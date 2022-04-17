use axum::{
    body::{Body, Bytes, HttpBody},
    headers::HeaderName,
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        HeaderValue, Request, StatusCode, Version,
    },
    response::IntoResponse,
    routing::any,
    Router,
};
use hyper::HeaderMap;
use std::{env, fs::File, io::Read, net::SocketAddr, path::PathBuf, str::FromStr};
use wasmer::{DeserializeError, Instance, Module, Store, Triple, VERSION};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_engine_universal::Universal;
use wasmer_wasi::{Pipe, WasiState};

const SERVER_SOFTWARE: &str = "wgi";

struct App<'a> {
    wasm: Vec<u8>,
    input: &'a [u8],
    vars: Vec<(String, String)>,
}

impl<'a> App<'a> {
    fn run(&self) -> anyhow::Result<String> {
        let store = Store::new(&Universal::new(Cranelift::default()).engine());
        let module = get_module_from_cache(&store, &self.wasm)?;

        let input = Pipe::new();
        let output = Pipe::new();

        let mut builder = WasiState::new("wgi-bin");
        builder.stdin(Box::new(input));
        builder.stdout(Box::new(output));
        builder.preopen_dir(".")?;
        for (key, value) in &self.vars {
            builder.env(key, value);
        }

        let mut wasi_env = builder.finalize()?;

        let import_object = wasi_env.import_object(&module)?;
        let instance = Instance::new(&module, &import_object)?;

        {
            let mut state = wasi_env.state();
            let wasi_stdin = state.fs.stdin_mut()?.as_mut().unwrap();
            wasi_stdin.write_all(self.input)?;
        }

        let run = instance.exports.get_native_function::<(), ()>("_start")?;
        run.call()?;

        let mut state = wasi_env.state();
        let wasi_stdout = state.fs.stdout_mut()?.as_mut().unwrap();
        let mut buf = String::new();
        wasi_stdout.read_to_string(&mut buf)?;

        Ok(buf)
    }
}

fn iter_path_splits(mut path: &str) -> impl Iterator<Item = (&str, &str)> {
    if path.as_bytes().get(0) == Some(&b'/') {
        path = &path[1..];
    }

    path.bytes()
        .enumerate()
        .filter(|(_, b)| *b == b'/')
        .map(|(i, _)| (&path[..i], &path[i..]))
        .chain(std::iter::once((path, "")))
}

fn server_protocol(version: Version) -> Option<&'static str> {
    match version {
        Version::HTTP_09 => Some("HTTP/0.9"),
        Version::HTTP_10 => Some("HTTP/1.0"),
        Version::HTTP_11 => Some("HTTP/1.1"),
        Version::HTTP_2 => Some("HTTP/2.0"),
        Version::HTTP_3 => Some("HTTP/3.0"),
        _ => None,
    }
}

fn to_cgi_http_header(header: &str) -> String {
    "HTTP_".to_string() + &header.to_ascii_uppercase().replace('-', "_")
}

async fn root(mut request: Request<Body>) -> impl IntoResponse {
    let path = request.uri().path();

    let mut wasm = Vec::new();
    let mut script_name = None;
    let mut path_info = None;

    for (path, rest) in iter_path_splits(path) {
        match File::open(path).and_then(|mut file| file.read_to_end(&mut wasm)) {
            Ok(_) => {
                script_name = Some(path);
                path_info = Some(rest);
                break;
            }
            Err(_) => continue,
        }
    }

    let query = request.uri().query();
    let method = format!("{}", request.method());

    let mut vars = vec![
        ("GATEWAY_INTERFACE".into(), "CGI/1.1".into()),
        ("SERVER_SOFTWARE".into(), SERVER_SOFTWARE.into()),
        ("SERVER_NAME".into(), "127.0.0.1".into()),
        ("SERVER_PORT".into(), "9000".into()),
        (
            "SERVER_PROTOCOL".into(),
            server_protocol(request.version())
                .expect("Unknown HTTP version")
                .into(),
        ),
        ("REQUEST_METHOD".into(), method),
        ("QUERY_STRING".into(), query.unwrap_or("").into()),
        // ("REMOTE_HOST".into(), "todo".into()),
    ];

    if let Some(var) = path_info {
        let translated = env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap()
            + var;

        vars.push(("PATH_INFO".into(), var.to_string()));
        vars.push(("PATH_TRANSLATED".into(), translated));
    }

    if let Some(var) = script_name {
        vars.push(("SCRIPT_NAME".into(), "/".to_string() + var));
    }

    vars.extend(request.headers().iter().map(|(header, value)| {
        let value = value.to_str().unwrap();

        // CGI handles two HTTP headers specially. The CGI RFC also suggests we should not
        // duplicate them with the HTTP_ prefix.
        match header {
            &CONTENT_TYPE => ("CONTENT_TYPE".into(), value.into()),
            &CONTENT_LENGTH => ("CONTENT_LENGTH".into(), value.into()),
            header => (to_cgi_http_header(header.as_str()), value.into()),
        }
    }));

    let body = if let Some(res) = request.body_mut().data().await {
        res.unwrap()
    } else {
        Bytes::new()
    };

    let app = App {
        wasm,
        input: &body,
        vars,
    };
    let output = app.run().unwrap();

    let mut status = StatusCode::OK;
    let mut headers = HeaderMap::new();

    if let Some((header, body)) = output.split_once("\n\n") {
        for line in header.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim_start();
                if key == "Status" {
                    status = value.parse().unwrap();
                } else {
                    headers.insert(
                        HeaderName::from_str(key).unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
            }
        }

        (status, headers, body.to_string())
    } else {
        (status, headers, output)
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/*path", any(root));

    let addr = SocketAddr::from(([127, 0, 0, 1], 9000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn get_module_from_cache(store: &Store, contents: &[u8]) -> anyhow::Result<Module> {
    let mut cache = get_cache()?;

    let hash = Hash::generate(contents);
    match unsafe { cache.load(store, hash) } {
        Ok(module) => Ok(module),
        Err(e) => {
            match e {
                DeserializeError::Io(_) => {}
                err => {
                    eprintln!("cached module is corrupted: {}", err);
                }
            }
            let module = Module::new(store, &contents)?;

            cache.store(hash, &module)?;
            Ok(module)
        }
    }
}

fn get_cache() -> anyhow::Result<FileSystemCache> {
    let cache_dir_root = get_cache_dir();
    let mut cache = FileSystemCache::new(cache_dir_root)?;

    let extension =
        wasmer_engine_universal::UniversalArtifact::get_default_extension(&Triple::host());

    cache.set_cache_extension(Some(extension));
    Ok(cache)
}

pub fn get_cache_dir() -> PathBuf {
    match env::var("WCGI_CACHE_DIR") {
        Ok(dir) => {
            let mut path = PathBuf::from(dir);
            path.push(VERSION);
            path
        }
        Err(_) => {
            let mut temp_dir = env::temp_dir();
            temp_dir.push("wcgi");
            temp_dir.push(VERSION);
            temp_dir
        }
    }
}

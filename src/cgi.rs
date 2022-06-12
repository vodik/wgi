use crate::wasm;
use axum::{
    body::{Body, Bytes, HttpBody},
    headers::HeaderName,
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        HeaderValue, Request, StatusCode, Version,
    },
    response::IntoResponse,
};
use hyper::HeaderMap;
use std::{env, fs::File, io::Read, str::FromStr};

const SERVER_SOFTWARE: &str = "wgi";

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

fn to_cgi_http_header(header: &HeaderName) -> String {
    let header = header.as_str();
    "HTTP_".to_string() + &header.to_ascii_uppercase().replace('-', "_")
}

pub async fn handler(mut request: Request<Body>) -> impl IntoResponse {
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
            header => (to_cgi_http_header(&header), value.into()),
        }
    }));

    let body = request
        .body_mut()
        .data()
        .await
        .transpose()
        .unwrap()
        .unwrap_or_else(Bytes::new);

    let output = wasm::App::new(wasm).run(&body, &vars).unwrap();

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

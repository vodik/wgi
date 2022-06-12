use std::{env, io::Read, path::PathBuf};
use wasmer::{DeserializeError, Instance, Module, Store, Triple, VERSION};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_engine_universal::Universal;
use wasmer_wasi::{Pipe, WasiState};

pub struct App<'a> {
    pub wasm: Vec<u8>,
    pub input: &'a [u8],
    pub vars: Vec<(String, String)>,
}

impl<'a> App<'a> {
    pub fn run(&self) -> anyhow::Result<String> {
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
    let path = env::var("WCGI_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir().join("wcgi"));

    path.join(VERSION)
}

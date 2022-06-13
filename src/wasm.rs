use std::{env, io::Read, path::PathBuf};
use wasmer::{DeserializeError, Instance, Module, Store, Triple, VERSION};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_engine_universal::Universal;
use wasmer_wasi::{Pipe, WasiState};

pub struct App(Vec<u8>);

impl App {
    pub fn new(wasm: Vec<u8>) -> Self {
        Self(wasm)
    }

    fn module(&self) -> anyhow::Result<Module> {
        let hash = Hash::generate(&self.0);

        let store = Store::new(&Universal::new(Cranelift::default()).engine());
        let mut cache = get_cache()?;

        match unsafe { cache.load(&store, hash) } {
            Ok(module) => Ok(module),
            Err(e) => {
                match e {
                    DeserializeError::Io(_) => {}
                    err => {
                        eprintln!("cached module is corrupted: {}", err);
                    }
                }

                let module = Module::new(&store, &&self.0)?;
                cache.store(hash, &module)?;
                Ok(module)
            }
        }
    }

    pub fn run(&self, input: &[u8], vars: &[(String, String)]) -> anyhow::Result<String> {
        let module = self.module()?;

        let stdin = Pipe::new();
        let stdout = Pipe::new();

        let mut builder = WasiState::new("wgi-bin");
        builder.stdin(Box::new(stdin));
        builder.stdout(Box::new(stdout));
        builder.preopen_dir(".")?;
        for (key, value) in vars {
            builder.env(key, value);
        }

        let mut wasi_env = builder.finalize()?;

        {
            let mut state = wasi_env.state();
            let wasi_stdin = state.fs.stdin_mut()?.as_mut().unwrap();
            wasi_stdin.write_all(input)?;
        }

        let import_object = wasi_env.import_object(&module)?;
        let instance = Instance::new(&module, &import_object)?;
        let run = instance.exports.get_native_function::<(), ()>("_start")?;
        run.call()?;

        let mut state = wasi_env.state();
        let wasi_stdout = state.fs.stdout_mut()?.as_mut().unwrap();
        let mut buf = String::new();
        wasi_stdout.read_to_string(&mut buf)?;
        Ok(buf)
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

fn get_cache_dir() -> PathBuf {
    let path = env::var("WCGI_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir().join("wcgi"));

    path.join(VERSION)
}

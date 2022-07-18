use crate::{
    cgi::CgiResponse,
    lambda::{self, LambdaRequest, LambdaResponse},
};
use std::{
    env,
    io::{self, Read, Seek, Write},
    path::PathBuf,
};
use tracing::Level;
use wasmer::{ChainableNamedResolver, DeserializeError, Instance, Module, Store, Triple, VERSION};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_engine_universal::Universal;
use wasmer_vfs::FsError;
use wasmer_wasi::{Pipe, VirtualFile, WasiState};

pub trait Logger {
    fn log(&self, message: &[u8]);
}

#[derive(Debug)]
pub struct TracingLogger {
    level: Level,
}

impl TracingLogger {
    pub fn new(level: Level) -> Self {
        Self { level }
    }
}

impl Default for TracingLogger {
    fn default() -> Self {
        Self::new(Level::INFO)
    }
}

impl Logger for TracingLogger {
    fn log(&self, message: &[u8]) {
        let message = String::from_utf8_lossy(message);
        match self.level {
            Level::ERROR => tracing::error!("{}", message),
            Level::WARN => tracing::warn!("{}", message),
            Level::INFO => tracing::info!("{}", message),
            Level::DEBUG => tracing::debug!("{}", message),
            Level::TRACE => tracing::trace!("{}", message),
        }
    }
}

#[derive(Debug)]
pub struct LogForwarder<L: Logger> {
    logger: L,
    incomplete: Vec<u8>,
}

impl<L> LogForwarder<L>
where
    L: Logger,
{
    pub fn new(logger: L) -> Self {
        Self {
            logger,
            incomplete: Vec::new(),
        }
    }
}

impl<L: Logger> Drop for LogForwarder<L> {
    fn drop(&mut self) {
        if !self.incomplete.is_empty() {
            self.logger.log(&self.incomplete);
        }
    }
}

impl<L: Logger> Read for LogForwarder<L> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "read only filesystem"))
    }
}

impl<L: Logger> Write for LogForwarder<L> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match buf.iter().position(|&c| c == b'\n') {
            Some(pos) => {
                let line = &buf[..pos];
                if self.incomplete.is_empty() {
                    self.logger.log(line)
                } else {
                    self.incomplete.extend(line);
                    self.logger.log(&self.incomplete);
                    self.incomplete.clear();
                }

                let mut rest = &buf[pos + 1..];
                while let Some(pos) = rest.iter().position(|&c| c == b'\n') {
                    self.logger.log(&rest[..pos]);
                    rest = &rest[pos + 1..];
                }

                self.incomplete.extend(rest);
            }
            None => self.incomplete.extend(buf),
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<L: Logger> Seek for LogForwarder<L> {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "can not seek in a pipe",
        ))
    }
}

impl<L: Logger + std::fmt::Debug + Send + 'static> VirtualFile for LogForwarder<L> {
    fn last_accessed(&self) -> u64 {
        0
    }

    fn last_modified(&self) -> u64 {
        0
    }

    fn created_time(&self) -> u64 {
        0
    }

    fn size(&self) -> u64 {
        self.incomplete.len() as u64
    }

    fn set_len(&mut self, len: u64) -> Result<(), FsError> {
        self.incomplete.resize(len as usize, 0);
        Ok(())
    }

    fn unlink(&mut self) -> Result<(), FsError> {
        Ok(())
    }

    fn bytes_available(&self) -> Result<usize, FsError> {
        Ok(self.incomplete.len())
    }
}

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

                let module = Module::from_binary(&store, &self.0)?;
                cache.store(hash, &module)?;
                Ok(module)
            }
        }
    }

    pub fn run_cgi(&self, input: &[u8], vars: &[(String, String)]) -> anyhow::Result<CgiResponse> {
        let module = self.module()?;

        let stdin = Pipe::new();
        let stdout = Pipe::new();
        let stderr = LogForwarder::new(TracingLogger::default());
        let mut builder = WasiState::new("wgi-bin");
        builder.stdin(Box::new(stdin));
        builder.stdout(Box::new(stdout));
        builder.stderr(Box::new(stderr));
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
        Ok(buf.parse()?)
    }

    pub fn run_lamba(&self, request: LambdaRequest) -> anyhow::Result<Option<LambdaResponse>> {
        let mut lambda_env = lambda::Env::new(request);
        let module = self.module()?;

        let stdout = LogForwarder::new(TracingLogger::default());
        let stderr = LogForwarder::new(TracingLogger::default());
        let mut builder = WasiState::new("lambda");
        builder.stdout(Box::new(stdout));
        builder.stderr(Box::new(stderr));
        builder.preopen_dir(".")?;

        let mut wasi_env = builder.finalize()?;
        let import_object = wasi_env.import_object(&module)?;

        let chained_imports = lambda_env.import_object(&module).chain_back(import_object);

        let instance = Instance::new(&module, &chained_imports)?;
        let start = instance.exports.get_native_function::<(), ()>("_start")?;
        start.call()?;

        let response = lambda_env.state().response.take();
        Ok(response)
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

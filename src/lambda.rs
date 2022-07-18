use crate::wasm;
use axum::{body::Body, http::Request, response::IntoResponse};
use hyper::Response;
use hyper::{
    http::header::{HeaderMap, HeaderName, HeaderValue},
    Method, Uri,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, Mutex, MutexGuard},
};
use wasmer::{
    imports, Array, Function, ImportObject, LazyInit, Memory, Module, WasmCell, WasmPtr, WasmerEnv,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LambdaRequest<'a> {
    resource: String,
    path: String,
    http_method: String,
    headers: HashMap<&'a str, Vec<&'a str>>,
    query_string_parameters: HashMap<&'a str, Vec<Cow<'a, str>>>,
    path_parameters: Option<String>,
    stage_variables: Option<String>,
    body: Option<Cow<'a, str>>,
    is_base64_encoded: bool,
}

impl<'a> LambdaRequest<'a> {
    fn from(method: &'a Method, uri: &'a Uri, headermap: &'a HeaderMap, body: &'a [u8]) -> Self {
        let query = uri.query();

        let mut headers: HashMap<&str, Vec<&str>> = Default::default();
        for key in headermap.keys() {
            let values: Result<Vec<&str>, _> = headermap
                .get_all(key)
                .iter()
                .map(|value| value.to_str())
                .collect();
            headers.insert(key.as_str(), values.unwrap());
        }

        let mut query_string_parameters: HashMap<&str, Vec<Cow<str>>> = Default::default();
        if let Some(query) = query {
            let qs: Vec<(&str, Cow<str>)> = serde_urlencoded::from_str(query).unwrap();

            for (key, value) in qs {
                query_string_parameters
                    .entry(key)
                    .or_insert_with(|| vec![value]);
            }
        }

        let (body, is_base64_encoded) = match std::str::from_utf8(body) {
            Ok(payload) => (payload.into(), false),
            Err(_) => (base64::encode(body).into(), true),
        };

        let path = uri.path().to_string();
        Self {
            resource: path.clone(),
            path,
            http_method: method.to_string(),
            headers,
            query_string_parameters,
            path_parameters: Default::default(),
            stage_variables: Default::default(),
            body: Some(body),
            is_base64_encoded,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LambdaResponse {
    status_code: u16,
    #[serde(default)]
    headers: HashMap<String, Vec<String>>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    is_base64_encoded: bool,
}

impl From<LambdaResponse> for Response<Body> {
    fn from(response: LambdaResponse) -> Self {
        let mut builder = Response::builder().status(response.status_code);

        let headers = builder.headers_mut().unwrap();
        for (key, values) in response.headers {
            for value in values {
                headers.append(
                    HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    HeaderValue::from_bytes(value.as_bytes()).unwrap(),
                );
            }
        }

        let body = response.body.map_or_else(Body::empty, |body| {
            if response.is_base64_encoded {
                Body::from(base64::decode(&body).unwrap())
            } else {
                Body::from(body)
            }
        });

        builder.body(body).unwrap()
    }
}

impl IntoResponse for LambdaResponse {
    fn into_response(self) -> axum::response::Response {
        let response: Response<Body> = self.into();
        response.into_response()
    }
}

#[derive(Debug)]
pub struct LambdaState {
    request: Vec<u8>,
    pub(crate) response: Option<LambdaResponse>,
}

#[derive(WasmerEnv, Clone)]
pub struct Env {
    state: Arc<Mutex<LambdaState>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

impl Env {
    pub fn new(request: LambdaRequest) -> Self {
        let request = serde_json::to_vec(&request).unwrap();
        let state = LambdaState {
            request,
            response: None,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
            memory: LazyInit::new(),
        }
    }

    pub fn state(&self) -> MutexGuard<LambdaState> {
        self.state.lock().unwrap()
    }

    pub fn memory(&self) -> &Memory {
        self.memory_ref()
            .expect("Memory should be set on `WasiEnv` first")
    }

    pub fn import_object(&mut self, module: &Module) -> ImportObject {
        let store = module.store();
        imports! {
            "lambda0" => {
                // "lambda_next" => Function::new_native_with_env(store, env.clone(), lambda::next),
                "lambda_event" => Function::new_native_with_env(store, self.clone(), event),
                "lambda_event_size" => Function::new_native_with_env(store, self.clone(), event_size),
                "lambda_send_response" => Function::new_native_with_env(store, self.clone(), send_response),
            }
        }
    }
}

fn copy_to_wasm(wasm: &[WasmCell<u8>], data: &[u8]) -> u32 {
    let mut nbytes = 0;
    for (byte, cell) in data.iter().zip(wasm.iter()) {
        cell.set(*byte);
        nbytes += 1;
    }
    nbytes
}

fn copy_from_wasm(wasm: &[WasmCell<u8>]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(wasm.len());
    for cell in wasm {
        buf.push(cell.get());
    }
    buf
}

pub fn event(env: &Env, buf: WasmPtr<u8, Array>, buf_len: u32) -> u32 {
    let memory = env.memory();
    let state = env.state();
    let buf = buf.deref(memory, 0, buf_len).unwrap();
    copy_to_wasm(&buf, &state.request)
}

pub fn event_size(env: &Env) -> u32 {
    let state = env.state();
    state.request.len().try_into().unwrap()
}

pub fn send_response(env: &Env, buf: WasmPtr<u8, Array>, buf_len: u32) -> i32 {
    let memory = env.memory();
    let buf = buf.deref(memory, 0, buf_len).unwrap();

    let event = copy_from_wasm(&buf);
    match serde_json::from_slice::<LambdaResponse>(&event) {
        Ok(value) => {
            let mut state = env.state();
            state.response = Some(value);
            0
        }
        Err(err) => {
            eprintln!("Failed to parse: {}", err);
            -1
        }
    }
}

fn iter_path_splits(mut path: &str) -> impl Iterator<Item = (&str, &str)> {
    if path.as_bytes().get(0) == Some(&b'/') {
        path = &path[1..];
    }

    path.bytes()
        .enumerate()
        .filter(|&(_, b)| b == b'/')
        .map(|(i, _)| (&path[..i], &path[i..]))
        .chain(std::iter::once((path, "")))
}

pub async fn handler(request: Request<Body>) -> impl IntoResponse {
    let path = request.uri().path();

    let mut wasm = Vec::new();
    // let mut script_name = None;
    // let mut path_info = None;
    let mut last_err = None;

    for (path, _rest) in iter_path_splits(path) {
        match File::open(path).and_then(|mut file| file.read_to_end(&mut wasm)) {
            Ok(_) => {
                // script_name = Some(path);
                // path_info = Some(rest);
                last_err = None;
                break;
            }
            Err(err) => {
                last_err = Some(err);
                continue;
            }
        }
    }

    if let Some(err) = last_err {
        panic!("failed to load file: {:?}", err);
    }

    let app = wasm::App::new(wasm);

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    let body = request.into_body();
    let body = hyper::body::to_bytes(body).await.unwrap().to_vec();

    let request = LambdaRequest::from(&method, &uri, &headers, &body);
    let response = app.run_lamba(request).unwrap();
    response.unwrap()
}

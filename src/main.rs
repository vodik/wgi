// extern crate wasmer_types as wasmer;

mod cgi;
mod lambda;
mod wasm;

use axum::{routing::any, Router};
use std::{env, net::SocketAddr};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

#[derive(Debug)]
enum Mode {
    Cgi,
    Lambda,
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // let fmt_layer = fmt::layer().json();
    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

#[tokio::main]
async fn main() {
    install_tracing();

    let mode = if env::var("WGI_MODE").map_or(false, |var| var == "lambda") {
        Mode::Lambda
    } else {
        Mode::Cgi
    };

    let app = Router::new()
        .route(
            "/*path",
            match mode {
                Mode::Cgi => any(cgi::handler),
                Mode::Lambda => any(lambda::handler),
            },
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 9000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

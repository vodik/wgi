mod cgi;
mod wasm;

use axum::{routing::any, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/*path", any(cgi::handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 9000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

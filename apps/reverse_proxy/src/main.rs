//! An example of how to run a webserver

use axum::{extract::Path, routing::get, Router};
use log::debug;
use serde::Deserialize;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod app_logging;

const ACAP_NAME: &str = "reverse_proxy";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Group {
    Admin,
    Operator,
    Viewer,
}

async fn hello_authenticated_user(Path(group): Path<Group>) -> String {
    format!("Hello {group:?}")
}

fn new_app() -> Router {
    Router::new()
        .route(
            &format!("/local/{ACAP_NAME}/api/:group"),
            get(hello_authenticated_user),
        )
        .layer(
            TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true)),
        )
}

#[tokio::main]
async fn main() {
    app_logging::init_logger();
    let app = new_app();
    debug!("Serving using TCP");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:2001")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

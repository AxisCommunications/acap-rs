use std::{env, net::SocketAddr, ops::Deref};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::{actors::placeholder, configuration::write_config, state::AppState, Message};

struct ServerError(anyhow::Error);
// Generally errors should not be automatically converted to `impl IntoResponse` because then it
// is easy to accidentally convert errors to the wrong response; `serde_json::Error` can be both
// a client error and a server error.
// But once an error has been converted to `anyhow::Error` it is probably intended to become a
// server error.
// TODO: Consider removing this conversion
impl From<anyhow::Error> for ServerError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let e = self.0;
        error!("Internal server error: {e:?}");
        if cfg!(debug_assertions) {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")).into_response()
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

impl ServerError {
    pub fn from_anyhow<E: Into<anyhow::Error>>(e: E) -> Self {
        Self(e.into())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self { port: 2001 }
    }
}

/// The simplest possible handler to help debug when things are not working.
async fn hello() -> &'static str {
    // The trailing newline is important for tools that assume POSIX lines, such as `curl`, to
    // properly display the response.
    "Bonjour!\n"
}

// Interact with the placeholder agent.
async fn interact(State(state): State<AppState>) -> Result<StatusCode, ServerError> {
    state
        .bus_tx
        .send(Message::PlaceholderIn)
        .map_err(ServerError::from_anyhow)?;
    Ok(StatusCode::ACCEPTED)
}

// TODO: Explore better configuration interfaces; in particular how to patch the config.
// TODO: Explore reloading agents instead of restarting the whole program.
#[derive(Deserialize)]
struct OverwriteConfig {
    placeholder: placeholder::Config,
}
async fn overwrite_config(
    State(state): State<AppState>,
    Json(query): Json<OverwriteConfig>,
) -> Result<StatusCode, ServerError> {
    let mut root_cfg = state.root_config.deref().clone();
    root_cfg.placeholder = Some(query.placeholder);
    write_config(&root_cfg)?;
    if let Some(tx) = state.stop_tx.lock().unwrap().take() {
        tx.send(()).unwrap()
    }
    Ok(StatusCode::ACCEPTED)
}

fn new_app(state: AppState) -> Router {
    // TODO: Explore nesting services to avoid repeating `/local/{app_name}`.
    let app_name = env::current_exe().unwrap();
    let app_name = app_name.file_name().unwrap().to_str().unwrap();
    let app = Router::new()
        .route(&format!("/local/{app_name}/api/v0/hello"), get(hello))
        .route(
            &format!("/local/{app_name}/api/v0/interact"),
            post(interact),
        )
        .route(
            &format!("/local/{app_name}/api/v0/config"),
            post(overwrite_config),
        );
    // No Axis devices are x86_64, so as long as this continues to be the case this will not be
    // erroneously included. However, even though the SDK only supports x86_64 hosts, this app
    // does not depend on the C APIs and could be built without the SDK. If that is done one a
    // host other than x86_64 this will be erroneously excluded.
    // TODO: Find a more robust configuration
    #[cfg(target_arch = "x86_64")]
    let app = {
        use tower_http::services::ServeDir;
        app.nest_service(
            &format!("/local/{app_name}"),
            ServeDir::new(format!("apps/{app_name}/html")),
        )
    };
    app.layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true)),
    )
    .with_state(state)
}

/// Actor that runs the web server.
///
/// Even when it passes no messages viewing it as an actor is convenient for the select! macro in
/// main.
pub struct Server {
    app_state: AppState,
}

impl Server {
    pub fn from_app_state(app_state: AppState) -> Self {
        Self { app_state }
    }
    pub async fn run(self) -> anyhow::Result<()> {
        let port = self.app_state.root_config.web.port;
        let rx = self.app_state.stop_rx.lock().unwrap().take().unwrap();

        debug!("Starting server on port {port}");
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, new_app(self.app_state))
            .with_graceful_shutdown(async {
                rx.await.ok();
                debug!("Stopping app")
            })
            .await?;
        debug!("App stopped");
        Ok(())
    }
}

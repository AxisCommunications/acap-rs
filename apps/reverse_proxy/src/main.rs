#![forbid(unsafe_code)]
//! An example of how to run a webserver

use std::{
    convert::Infallible,
    fs, io,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use anyhow::Context;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{net::UnixListener, time::sleep};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AccessPolicy {
    Admin,
    Operator,
    Viewer,
    Anonymous,
}

async fn whoami(Path(policy): Path<AccessPolicy>) -> impl IntoResponse {
    match policy {
        AccessPolicy::Admin => "admin",
        AccessPolicy::Operator => "operator",
        AccessPolicy::Viewer => "viewer",
        AccessPolicy::Anonymous => "anonymous",
    }
}

async fn ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|s| async {
        let (sender, receiver) = s.split();
        tokio::select!(
            Err(e) = publish_time(sender) => error!("Closing websocket because {e:?}"),
            _ = discard_inbound(receiver) => {},
        );
    })
}

async fn publish_time(
    mut stream: SplitSink<WebSocket, Message>,
) -> Result<Infallible, axum::Error> {
    loop {
        // Unwrap is OK `now` is always after `UNIX_EPOCH` on well-configured systems.
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        stream.send(Message::text(format!("{now}"))).await?;
        sleep(Duration::from_secs(1)).await
    }
}

async fn discard_inbound(mut stream: SplitStream<WebSocket>) {
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Text(msg)) => warn!("Discarding inbound text {msg}"),
            Ok(Message::Binary(_)) => warn!("Discarding inbound binary"),
            Ok(Message::Ping(_)) => {}
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(msg)) => info!("Client is closing the connection {msg:?}"),
            Err(e) => {
                warn!("Failed to discard inbound because {e}");
                break;
            }
        }
    }
}

fn new_app() -> Router {
    let app = Router::new()
        .route(
            concat!("/local/", env!("CARGO_PKG_NAME"), "/api/{policy}/whoami"),
            get(whoami),
        )
        .route(
            concat!("/local/", env!("CARGO_PKG_NAME"), "/api/{policy}/ws"),
            get(ws),
        );

    // No Axis devices are x86_64, so as long as this continues to be the case this will not be
    // erroneously included. However, even though the SDK only supports x86_64 hosts, this app
    // does not depend on the C APIs and could be built without the SDK. If that is done one a
    // host other than x86_64 this will be erroneously excluded.
    // TODO: Find a more robust configuration
    #[cfg(any(target_arch = "x86_64", target_os = "macos"))]
    let app = {
        use tower_http::services::ServeDir;
        app.nest_service(
            concat!("/local/", env!("CARGO_PKG_NAME")),
            ServeDir::new(concat!("apps/", env!("CARGO_PKG_NAME"), "/html")),
        )
    };

    app.layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true)),
    )
}

pub fn bind_uds_listener(path: &std::path::Path) -> anyhow::Result<UnixListener> {
    let directory = path.parent().context("Socket path has no parent")?;
    fs::create_dir_all(directory).context("Failed to create socket directory")?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o755))
        .context("Failed to set socket directory permissions")?;

    match fs::remove_file(path) {
        Ok(()) => debug!("Removed old socket file"),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("Failed to remove old socket file"),
    }

    let listener = UnixListener::bind(path).context("Failed to bind socket file")?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o666))
        .context("Failed to set permissions on socket file")?;

    Ok(listener)
}

#[tokio::main]
async fn main() {
    acap_logging::init_logger();
    let app = new_app();
    // Unwrap is OK because if we cannot start the web server then there is nothing useful the app
    // can do, so exiting is appropriate.
    if cfg!(any(target_arch = "x86_64", target_os = "macos")) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:2001")
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        // Keep the path in sync with the targets declared in `manifest.json`.
        let path = PathBuf::from(concat!("/run/http/", env!("CARGO_PKG_NAME"), "/app.sock"));
        let listener = bind_uds_listener(&path).unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

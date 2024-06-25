//! An example of how to run a webserver

use std::{
    convert::Infallible,
    time::{Duration, SystemTime},
};

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
use tokio::time::sleep;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

const APP_NAME: &str = "reverse_proxy";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Admin,
    Operator,
    Viewer,
    Anonymous,
}

async fn whoami(Path(scope): Path<Scope>) -> impl IntoResponse {
    match scope {
        Scope::Admin => "admin",
        Scope::Operator => "operator",
        Scope::Viewer => "viewer",
        Scope::Anonymous => "anonymous",
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
        stream.send(Message::Text(format!("{now}"))).await?;
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
        .route(&format!("/local/{APP_NAME}/api/:scope/whoami"), get(whoami))
        .route(&format!("/local/{APP_NAME}/api/:scope/ws"), get(ws));

    // No Axis devices are x86_64, so as long as this continues to be the case this will not be
    // erroneously included. However, even though the SDK only supports x86_64 hosts, this app
    // does not depend on the C APIs and could be built without the SDK. If that is done one a
    // host other than x86_64 this will be erroneously excluded.
    // TODO: Find a more robust configuration
    #[cfg(target_arch = "x86_64")]
    let app = {
        use tower_http::services::ServeDir;
        app.nest_service(
            &format!("/local/{APP_NAME}"),
            ServeDir::new(format!("apps/{APP_NAME}/otherfiles/html")),
        )
    };

    app.layer(
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

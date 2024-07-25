use crate::{
    actors::{placeholder::Placeholder, server::Server},
    configuration::read_config,
};

mod actors;
mod configuration;
mod state;

// TODO: Improve how actors are connected
// Problems include
// * Not reproducible, especially if an adapter is replaced with something that replays messages
//   from a file.
// * High contention around the crate::common::model::Message type.
#[derive(Clone, Debug)]
enum Message {
    PlaceholderIn,
    PlaceholderOut,
}

#[tokio::main]
async fn main() {
    acap_logging::init_logger();

    let app_state = state::AppState::new(read_config());

    let placeholder = Placeholder::from_app_state(app_state.clone()).run();
    let server = Server::from_app_state(app_state.clone()).run();

    // TODO: Investigate if this is good way to stop actors.
    tokio::select!(
        r = placeholder => r.unwrap(),
        r = server => r.unwrap(),
    );
}

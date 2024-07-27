use crate::{
    actors::{mqtt_broker::MqttBroker, mqtt_client::MqttClient},
    configuration::read_config,
};

mod actors;
mod configuration;

pub mod broker;
pub mod client;
pub mod hush;
mod state;

#[tokio::main]
async fn main() {
    // TODO: Consider finding a way to filter the excessively verbose logs from paho-mqtt.
    acap_logging::init_logger();

    let app_state = state::AppState::new(read_config().unwrap());

    let mqtt_broker = MqttBroker::from_app_state(app_state.clone()).run();
    let mqtt_client = MqttClient::from_app_state(app_state.clone()).run();

    tokio::select!(
        r = mqtt_broker => r.unwrap(),
        r = mqtt_client => r.unwrap(),
    );
}

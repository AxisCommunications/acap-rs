#![forbid(unsafe_code)]
//! An example of how to subscribe to an ONVIF event. It requires the `send_event` example to be
//! installed and running as it relies on those events to receive and log data.

use axevent::flex::{Handler, KeyValueSet, Subscription};
use log::error;
use log::info;

fn onviftrigger_subscription(handler: &Handler, token: u32) -> anyhow::Result<Subscription> {
    let mut key_value_set = KeyValueSet::new();

    // Set keys and namespaces for the event to be subscribed
    key_value_set
        .add_key_value(c"topic0", Some(c"tns1"), Some(c"Monitoring"))?
        .add_key_value(c"topic1", Some(c"tns1"), Some(c"ProcessorUsage"))?;

    let _subscription = handler.subscribe(key_value_set, |_subscription, event| {
        match event.key_value_set().get_double(c"Value", None) {
            Ok(value) => {
                info!("Received event with value: {value:?}");
            }
            Err(e) => {
                error!("Error {}", e);
            }
        }
    })?;

    info!("And here is the token: {}", token);

    Ok(_subscription)
}

fn main() {
    acap_logging::init_logger();

    let handler = Handler::new();
    info!("Started logging from subscribe event application");
    onviftrigger_subscription(&handler, 1234).unwrap();

    let main_loop = glib::MainLoop::new(None, false);
    main_loop.run();
}

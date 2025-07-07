#![forbid(unsafe_code)]
use std::sync::Arc; // A thread-safe reference-counting pointer
use axevent::flex::{Subscription, Handler, KeyValueSet};
use log::info;
use log::error;

fn onviftrigger_subscription(handler: Arc<Handler>, token: u32) -> anyhow::Result<Subscription> {
    let mut key_value_set = KeyValueSet::new();

    // let handler = Arc::new(handler);
    // Set keys and namespaces for the event to be subscribed
    key_value_set
        .add_key_value(c"topic0", Some(c"tns1"), Some(c"Monitoring"))?
        .add_key_value(c"topic1", Some(c"tns1"), Some(c"ProcessorUsage"))?;

    let _subscription = handler.subscribe(key_value_set, move |_subscription, event | {
        match event.key_value_set().get_double(c"Value", None) {
            Ok(value) => {
                info!("Received event with value: {}", value);
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

    let handler = Arc::new(Handler::new());
    let main_loop = glib::MainLoop::new(None, false);

    info!("Started logging from subscribe event application");
    onviftrigger_subscription(handler.clone(), 1234).unwrap();

    main_loop.run();
}

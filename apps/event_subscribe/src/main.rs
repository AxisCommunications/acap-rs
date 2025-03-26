#![forbid(unsafe_code)]
//! An example of how to subscribe to manual trigger events using `axevent::nonblock`
use axevent::flex::{Handler, KeyValueSet};
use axevent::nonblock::Subscription;
use glib::MainContext;
use log::{info, warn};

use futures_lite::StreamExt;

async fn app() -> anyhow::Result<()> {
    let mut subscription_template = KeyValueSet::new();
    subscription_template
        .add_key_value(c"topic0", Some(c"tns1"), Some(c"Device"))?
        .add_key_value(c"topic1", Some(c"tnsaxis"), Some(c"IO"))?
        .add_key_value(c"topic2", Some(c"tnsaxis"), Some(c"VirtualPort"))?
        .add_key_value::<i32>(c"port", None, None)?
        .add_key_value::<bool>(c"state", None, None)?;
    let handler = Handler::new();
    let mut manual_trigger_events = Subscription::try_new(&handler, subscription_template)?;
    while let Some(evt) = manual_trigger_events.next().await {
        info!(
            "Got manual trigger event on port {} with state {}",
            evt.key_value_set().get_integer(c"port", None)?,
            evt.key_value_set().get_boolean(c"state", None)?
        );
    }
    Ok(())
}
async fn app_infallible() {
    if let Err(e) = app().await {
        warn!("Unexpected error when running app: {e}");
    }
}

fn main() {
    acap_logging::init_logger();
    let ctx = MainContext::default();

    let main_loop = glib::MainLoop::new(Some(&ctx), false);
    ctx.spawn_local(app_infallible());
    main_loop.run();
}

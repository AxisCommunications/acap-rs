#![forbid(unsafe_code)]
//! An example of how to subscribe to manual trigger events using `axevent::nonblock`
use anyhow::anyhow;
use std::pin::pin;

use axevent::flex::KeyValueSet;
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
    let mut manual_trigger_events = Subscription::default();
    manual_trigger_events
        .try_subscribe(subscription_template)
        .map_err(|e| anyhow!("Unable to subscribe due to {e}"))?;
    let mut manual_trigger_events = pin!(manual_trigger_events);
    while let Some(evt) = manual_trigger_events.next().await {
        info!(
            "Got manual trigger event on port {}",
            evt.key_value_set().get_integer(c"port", None)?
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
    let _ = ctx.spawn_local(app_infallible());
    main_loop.run();
}

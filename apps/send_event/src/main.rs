#![forbid(unsafe_code)]
//! An example of how to send an ONVIF event periodically.
//!
//! The audience for this example is users who are familiar with the C APIs and have existing code
//! that they wish to port to Rust; this is probably not the most idiomatic way to send an event
//! in a greenfield Rust project.

use std::sync::Arc;

use axevent::flex::{Declaration, Event, Handler, KeyValueSet};
use log::info;

struct AppData {
    handler: Arc<Handler>,
    event_id: Declaration,
    value: f64,
}

fn send_event(app_data: &mut AppData) -> glib::ControlFlow {
    let mut key_value_set = KeyValueSet::new();
    let _ = key_value_set.add_key_value(c"Value", None, Some(app_data.value));
    let event = Event::new2(key_value_set, None);
    let _ = app_data.handler.send_event(event, &app_data.event_id);
    info!("Send stateful event with value {}", app_data.value);
    app_data.value = if app_data.value >= 100.0 {
        0.0
    } else {
        app_data.value + 10.0
    };
    glib::ControlFlow::Continue
}

fn declaration_complete(declaration: Declaration, handler: Arc<Handler>, start_value: f64) {
    let mut app_data = AppData {
        handler,
        event_id: declaration,
        value: start_value,
    };
    glib::timeout_add_seconds(10, move || send_event(&mut app_data));
}

fn setup_declaration(handler: Handler, start_value: f64) -> anyhow::Result<Declaration> {
    let handler = Arc::new(handler);
    let mut key_value_set = KeyValueSet::new();
    key_value_set
        .add_key_value(c"topic0", Some(c"tns1"), Some(c"Monitoring"))?
        .add_key_value(c"topic1", Some(c"tns1"), Some(c"ProcessorUsage"))?
        .add_key_value(c"Token", None, Some(0))?
        .add_key_value(c"Value", None, Some(start_value))?
        .mark_as_source(c"Token", None)?
        .mark_as_user_defined(c"Token", None, c"wstype:tt:ReferenceToken")?
        .mark_as_data(c"Value", None)?
        .mark_as_user_defined(c"Value", None, c"wstype:xs:float")?;
    let declaration = handler.declare(
        &key_value_set,
        false,
        Some({
            let mut handler = Some(Arc::clone(&handler));
            move |declaration| {
                if let Some(handler) = handler.take() {
                    declaration_complete(declaration, handler, start_value);
                }
            }
        }),
    )?;
    Ok(declaration)
}

fn main() {
    acap_logging::init_logger();

    let handler = Handler::new();
    setup_declaration(handler, 0.0).unwrap();

    let main_loop = glib::MainLoop::new(None, false);
    main_loop.run();
}

#[cfg(test)]
mod tests {
    use std::{ffi::CStr, time, time::Duration};

    use anyhow::bail;
    use axevent::{
        ergo::{date_time, system_time, Declaration, MainLoop, Subscription},
        flex::{CStringPtr, Event, Handler, KeyValueSet},
    };
    use log::{debug, LevelFilter};

    fn init() {
        let _ = env_logger::Builder::new()
            .filter_level(LevelFilter::Debug)
            .parse_default_env()
            .is_test(true)
            .try_init();
    }

    #[test]
    fn get_integer_none() {
        let mut kvs = axevent::flex::KeyValueSet::new();
        kvs.add_key_value::<i32>(c"foo", None, None).unwrap();
        assert_eq!(kvs.get_integer(c"foo", None).unwrap(), 0);
    }

    // thread 'tests::get_double_none' panicked at crates/axevent/src/flex.rs:605:22:
    // Expected gboolean to be either 0 or 1 but got 3
    #[test]
    fn get_boolean_none() {
        let mut kvs = axevent::flex::KeyValueSet::new();
        kvs.add_key_value::<f64>(c"foo", None, None).unwrap();
        assert_eq!(kvs.get_double(c"foo", None).unwrap(), 0.0);
    }

    #[test]
    fn get_double_none() {
        let mut kvs = axevent::flex::KeyValueSet::new();
        kvs.add_key_value::<bool>(c"foo", None, None).unwrap();
        assert_eq!(kvs.get_boolean(c"foo", None).unwrap(), false);
    }

    // thread 'tests::read_none_string' panicked at crates/axevent/src/flex.rs:80:9:
    // assertion failed: !ptr.is_null()
    #[test]
    fn get_string_none() {
        let mut kvs = axevent::flex::KeyValueSet::new();
        kvs.add_key_value::<&CStr>(c"foo", None, None).unwrap();
        assert_eq!(kvs.get_string(c"foo", None).unwrap().as_c_str(), c"");
    }

    fn topic() -> anyhow::Result<KeyValueSet> {
        let mut kvs = KeyValueSet::default();
        kvs.add_key_value(
            c"topic0",
            Some(c"tnsaxis"),
            Some(c"CameraApplicationPlatform"),
        )?;
        kvs.add_key_value(c"topic1", Some(c"tnsaxis"), Some(c"HelloAXEvent"))?;
        Ok(kvs)
    }

    fn send_and_receive_event(sent: &CStr) -> anyhow::Result<CStringPtr> {
        let main_loop = MainLoop::new();

        let handler = Handler::new();

        debug!("Subscribing to events");
        let subscription = Subscription::try_new(topic()?, &handler)?;

        debug!("Declaring event");
        let mut dec_kvs = topic()?;
        dec_kvs.add_key_value::<&CStr>(c"Greeting", None, None)?;
        let declaration = Declaration::try_new(dec_kvs, true, &handler)?;
        debug!("Waiting for declaration to be registered...");
        assert!(declaration.rx.recv_timeout(Duration::from_secs(5)).is_ok());

        debug!("Sending event");
        let now = time::SystemTime::now();
        let mut evt_kvs = KeyValueSet::new();
        evt_kvs.add_key_value(c"Greeting", None, Some(sent))?;
        declaration.send_event(Event::new2(evt_kvs, Some(date_time(now))))?;

        debug!("Waiting receive event...");
        let event = subscription.rx.recv_timeout(Duration::from_secs(5))?;

        // TODO: Investigate more precise timestamps
        assert_eq!(
            system_time(event.time_stamp2())
                .duration_since(time::UNIX_EPOCH)?
                .as_secs(),
            now.duration_since(time::UNIX_EPOCH)?.as_secs()
        );

        let kvs = event.key_value_set();
        assert_eq!(
            kvs.get_string(c"topic0", Some(c"tnsaxis"))?.as_c_str(),
            c"CameraApplicationPlatform"
        );
        assert_eq!(
            kvs.get_string(c"topic1", Some(c"tnsaxis"))?.as_c_str(),
            c"HelloAXEvent"
        );
        let received = kvs.get_string(c"Greeting", None)?;

        if let Err(e) = main_loop.quit_and_join() {
            bail!("Main loop exited with an error: {e:?}");
        }
        Ok(received)
    }

    #[test]
    fn can_send_and_receive_event() {
        // axevent supports keys and values that are not valid UTF-8;
        // using an invalid UTF-8 string is a low-effort way of ensuring that support is propagated.
        let expected = c"Hello\xc3\x28";

        // Sanity check
        assert!(expected.to_str().is_err());

        let actual = send_and_receive_event(expected).unwrap();
        assert_eq!(actual.as_c_str(), expected);
    }

    #[test]
    fn can_send_and_receive_stateful_boolean() -> anyhow::Result<()> {
        init();

        let main_loop = MainLoop::new();

        let handler = Handler::new();

        debug!("Subscribing to events...");
        let subscription = Subscription::try_new(topic()?, &handler)?;

        debug!("Declaring event...");
        let topic2 = c"active";
        let mut dec_kvs = topic()?;
        dec_kvs.add_key_value(topic2, None, Some(false))?;
        let declaration = Declaration::try_new(dec_kvs, true, &handler)?;
        debug!("Waiting for declaration to be registered...");
        assert!(declaration.rx.recv_timeout(Duration::from_secs(5)).is_ok());

        debug!("Activating event...");
        let mut activate_kvs = KeyValueSet::new();
        activate_kvs.add_key_value(topic2, None, Some(true))?;
        declaration.send_event(Event::new2(activate_kvs, None))?;

        debug!("Verifying active state...");
        let active = subscription
            .rx
            .recv_timeout(Duration::from_secs(5))?
            .key_value_set()
            .get_boolean(topic2, None)?;
        assert!(active);

        debug!("Deactivating event...");
        let mut inactivate_kvs = KeyValueSet::new();
        inactivate_kvs.add_key_value(topic2, None, Some(false))?;
        declaration.send_event(Event::new2(inactivate_kvs, None))?;

        debug!("Verifying inactive state...");
        let active = subscription
            .rx
            .recv_timeout(Duration::from_secs(5))?
            .key_value_set()
            .get_boolean(topic2, None)?;
        assert!(!active);

        if let Err(e) = main_loop.quit_and_join() {
            bail!("Main loop exited with an error: {e:?}");
        }
        Ok(())
    }

    #[test]
    fn can_declare_without_callback() {
        let handler = Handler::new();
        let declaration = handler
            .declare::<fn(axevent::flex::Declaration)>(&topic().unwrap(), true, None)
            .unwrap();
        // TODO: Consider refactoring the API to make it harder to make this mistake.
        handler.undeclare(&declaration).unwrap();
        let main_context = glib::MainContext::default();
        while main_context.iteration(false) {}
    }
}

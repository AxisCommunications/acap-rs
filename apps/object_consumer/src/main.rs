//! An example app that subscribes to object detection data using [`datahub_sys`].
//!
//! Re-implements the [C example for consuming Device Data Hub data].
//! Unlike the C example, which relies on globals and `atexit`, resources are owned by `main` and
//! released explicitly on every exit path.
//!
//! [C example for consuming Device Data Hub data]: https://github.com/AxisCommunications/acap-native-sdk-examples/tree/main/device-data-hub/acap-communication/object-consumer

use std::{
    ffi::{c_char, c_int, c_void, CStr},
    process::ExitCode,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use datahub_sys::{
    dh_client_connect, dh_client_create, dh_client_create_subscriber, dh_client_destroy,
    dh_client_disconnect, dh_client_set_logging, dh_error_destroy, dh_error_to_string,
    dh_filter_add_topic_name, dh_filter_create, dh_filter_destroy, dh_subscribe_options_add_filter,
    dh_subscribe_options_create, dh_subscribe_options_destroy,
    dh_subscribe_options_set_enable_data_updates, dh_subscriber_destroy,
    dh_subscriber_set_data_callback, dh_subscriber_subscribe, dh_topic_data_get_json_data,
    dh_topic_sample_get_data, DHClient, DHError, DHSubscriber, DHTopicSample, DH_LOG_INFO,
    DH_LOG_TARGET_CONSOLE,
};
use libc::{SIGINT, SIGTERM};
use log::{error, info};

const TOPIC_NAME: &CStr = c"com.example.objectdetector";
const USER_DATA: &CStr = c"object_consumer_data";

static KEEP_RUNNING: AtomicBool = AtomicBool::new(true);

/// Logs and destroys `error`, returning `true`, if it is set; returns `false` otherwise.
unsafe fn handle_client_error(error: *mut DHError, context: &str) -> bool {
    if error.is_null() {
        return false;
    }
    let message = CStr::from_ptr(dh_error_to_string(error)).to_string_lossy();
    error!("Error in {context}: {message}");
    let () = dh_error_destroy(error);
    true
}

/// Creates a client and connects it to the Device Data Hub.
unsafe fn initialize_client() -> Option<*mut DHClient> {
    let mut error: *mut DHError = ptr::null_mut();
    let client = dh_client_create(c"Client for object_consumer".as_ptr(), &mut error);
    if client.is_null() {
        let _ = handle_client_error(error, "create client");
        return None;
    }

    let mut error: *mut DHError = ptr::null_mut();
    if !dh_client_set_logging(client, DH_LOG_INFO, DH_LOG_TARGET_CONSOLE, &mut error) {
        let _ = handle_client_error(error, "set logging");
    }

    let mut error: *mut DHError = ptr::null_mut();
    let _ = dh_client_connect(client, &mut error);
    if handle_client_error(error, "client connect") {
        let () = dh_client_destroy(client);
        return None;
    }

    Some(client)
}

/// Logs the data of each received sample.
unsafe extern "C" fn on_data_received(sample: *const DHTopicSample, user_data: *mut c_void) {
    debug_assert!(!sample.is_null());
    debug_assert!(!user_data.is_null());
    let user_data = CStr::from_ptr(user_data.cast::<c_char>()).to_string_lossy();
    info!("User data: {user_data}");
    let topic_data = dh_topic_sample_get_data(sample);
    let data = dh_topic_data_get_json_data(topic_data);
    if !data.is_null() {
        let data = CStr::from_ptr(data).to_string_lossy();
        info!("Received Object Detection data: {data}");
    }
}

/// Creates a subscriber and subscribes to data updates for `topics`.
unsafe fn setup_subscription(client: *mut DHClient, topics: &[&CStr]) -> Option<*mut DHSubscriber> {
    debug_assert!(!client.is_null());
    let mut error: *mut DHError = ptr::null_mut();
    let subscriber = dh_client_create_subscriber(
        client,
        c"Data subscriber for object-consumer".as_ptr(),
        &mut error,
    );
    if handle_client_error(error, "create subscriber") {
        return None;
    }

    let mut error: *mut DHError = ptr::null_mut();
    let _ = dh_subscriber_set_data_callback(
        subscriber,
        Some(on_data_received),
        USER_DATA.as_ptr().cast_mut().cast(),
        &mut error,
    );
    if handle_client_error(error, "set data callback") {
        let () = dh_subscriber_destroy(subscriber);
        return None;
    }

    let filter = dh_filter_create();
    if filter.is_null() {
        error!("Failed to create filter");
        let () = dh_subscriber_destroy(subscriber);
        return None;
    }

    for topic in topics {
        let mut error: *mut DHError = ptr::null_mut();
        let _ = dh_filter_add_topic_name(filter, topic.as_ptr(), &mut error);
        if handle_client_error(error, "add topic name to filter") {
            let () = dh_filter_destroy(filter);
            let () = dh_subscriber_destroy(subscriber);
            return None;
        }
    }

    let options = dh_subscribe_options_create();
    if options.is_null() {
        error!("Failed to create subscription options");
        let () = dh_filter_destroy(filter);
        let () = dh_subscriber_destroy(subscriber);
        return None;
    }

    let mut error: *mut DHError = ptr::null_mut();
    let _ = dh_subscribe_options_add_filter(options, filter, &mut error);
    let () = dh_filter_destroy(filter);
    if handle_client_error(error, "add filter to options") {
        let () = dh_subscribe_options_destroy(options);
        let () = dh_subscriber_destroy(subscriber);
        return None;
    }

    let () = dh_subscribe_options_set_enable_data_updates(options, true);

    let mut error: *mut DHError = ptr::null_mut();
    let _ = dh_subscriber_subscribe(subscriber, options, &mut error);
    let () = dh_subscribe_options_destroy(options);
    if handle_client_error(error, "subscribe to topic") {
        let () = dh_subscriber_destroy(subscriber);
        return None;
    }

    Some(subscriber)
}

/// Destroys `subscriber`, if any, then disconnects and destroys `client`.
unsafe fn cleanup_resources(client: *mut DHClient, subscriber: *mut DHSubscriber) {
    debug_assert!(!client.is_null());
    if !subscriber.is_null() {
        let () = dh_subscriber_destroy(subscriber);
    }

    let mut error: *mut DHError = ptr::null_mut();
    let _ = dh_client_disconnect(client, &mut error);
    let _ = handle_client_error(error, "client disconnect");
    let () = dh_client_destroy(client);
}

extern "C" fn signal_handler(sig: c_int) {
    if sig == SIGINT || sig == SIGTERM {
        KEEP_RUNNING.store(false, Ordering::Relaxed);
    }
}

fn main() -> ExitCode {
    acap_logging::init_logger();
    info!("Application started");

    unsafe {
        let _ = libc::signal(SIGINT, signal_handler as libc::sighandler_t);
        let _ = libc::signal(SIGTERM, signal_handler as libc::sighandler_t);
    }

    let topics = [TOPIC_NAME];

    let Some(client) = (unsafe { initialize_client() }) else {
        return ExitCode::FAILURE;
    };
    let Some(subscriber) = (unsafe { setup_subscription(client, &topics) }) else {
        let () = unsafe { cleanup_resources(client, ptr::null_mut()) };
        return ExitCode::FAILURE;
    };

    while KEEP_RUNNING.load(Ordering::Relaxed) {
        let _ = unsafe { libc::pause() };
    }

    info!("Application terminated");
    let () = unsafe { cleanup_resources(client, subscriber) };
    ExitCode::SUCCESS
}

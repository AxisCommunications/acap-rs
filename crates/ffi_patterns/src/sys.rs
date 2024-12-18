use std::collections::HashMap;
use std::ffi::{c_int, c_void};

type Callback = unsafe extern "C" fn(*mut c_void);

struct Subscription {
    callback: Option<Callback>,
    user_data: *mut c_void,
}

pub struct Handler {
    next_key: i32,
    subscriptions: HashMap<i32, Subscription>,
}

pub extern "C" fn handler_new() -> *mut Handler {
    Box::into_raw(Box::new(Handler {
        next_key: 0,
        subscriptions: HashMap::new(),
    }))
}

pub unsafe extern "C" fn handler_free(handler: *mut Handler) {
    drop(Box::from_raw(handler));
}

pub unsafe extern "C" fn handler_run(handler: *mut Handler) {
    let handler = handler.as_mut().unwrap();
    for v in handler.subscriptions.values_mut() {
        for _ in 0..2 {
            if let Some(f) = v.callback {
                f(v.user_data);
            }
        }
    }
}

pub unsafe extern "C" fn handler_subscribe(
    handler: *mut Handler,
    callback: Option<Callback>,
    user_data: *mut c_void,
) -> i32 {
    let handler = handler.as_mut().unwrap();
    let key = handler.next_key;
    handler.next_key += 1;
    handler.subscriptions.insert(
        key,
        Subscription {
            callback,
            user_data,
        },
    );
    key
}

#[allow(non_camel_case_types)]
pub struct error_t {
    pub code: c_int,
}

pub unsafe extern "C" fn error_new() -> *mut error_t {
    Box::into_raw(Box::new(error_t { code: 42 }))
}

pub unsafe extern "C" fn error_free(error: *mut error_t) {
    drop(Box::from_raw(error))
}

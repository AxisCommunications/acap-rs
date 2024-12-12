use crate::sys;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::{spawn, JoinHandle};

/// A struct that calls a function when dropped.
/// This is intended to help make sure pointers are cleaned up at an appropriate time.
///
/// It is a verbatim copy of the same type in:
/// - `axevent`
/// - `mdb`
struct Deferred(Option<Box<dyn FnOnce()>>);

impl Deferred {
    unsafe fn new<T: 'static>(ptr: *mut T) -> Self {
        Self(Some(Box::new(move || drop(Box::from_raw(ptr)))))
    }
}

impl Drop for Deferred {
    fn drop(&mut self) {
        assert!(self.0.is_some());
        self.0.take().unwrap()()
    }
}

/// A struct on which callbacks can be registered.
///
/// This pattern is used in:
/// - `axevent`
/// - `mdb`
struct Handler {
    raw: *mut sys::Handler,
    callbacks: Mutex<HashMap<Subscription, Deferred>>,
}

impl Handler {
    fn new() -> Self {
        Self {
            raw: sys::handler_new(),
            callbacks: Mutex::new(HashMap::new()),
        }
    }

    fn spawn(&self) -> JoinHandle<()> {
        #[derive(Debug)]
        struct MyPtr(*mut sys::Handler);
        unsafe impl Send for MyPtr {}

        impl MyPtr {
            fn as_ptr(&self) -> *mut sys::Handler {
                self.0
            }
        }
        let ptr = MyPtr(self.raw);
        unsafe { spawn(move || sys::handler_run(ptr.as_ptr())) }
    }

    /// Add a callback freed automatically when the handler goes out of scope.
    ///
    /// This pattern is used in:
    /// - `axevent::flex::Handler::subscribe`
    /// - `mdb::SubscriberConfig::try_new` (soon)
    fn subscribe<F>(&self, callback: F) -> Subscription
    where
        F: FnMut() + Send + 'static,
    {
        let raw_callback = Box::into_raw(Box::new(callback));
        let callback = unsafe { Deferred::new(raw_callback) };
        unsafe {
            let handle = sys::handler_subscribe(
                self.raw,
                Some(Self::trampoline::<F>),
                raw_callback as *mut c_void,
            );
            let handle = Subscription(handle);
            self.callbacks.lock().unwrap().insert(handle, callback);
            handle
        }
    }

    /// Remove a callback before the handler is dropped.
    ///
    /// This pattern is used in:
    /// - `axevent::flex::Handler::subscribe`
    fn unsubscribe(&self, handle: &Subscription) {
        self.callbacks.lock().unwrap().remove(handle);
    }

    unsafe extern "C" fn trampoline<F>(user_data: *mut c_void)
    where
        F: FnMut(),
    {
        let callback = &mut *(user_data as *mut F);
        callback();
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        unsafe {
            sys::handler_free(self.raw);
        }
    }
}

unsafe impl Send for Handler {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Subscription(i32);

#[test]
fn transfer_callbacks() {
    let handler = Handler::new();

    static SHARED_COUNT: AtomicUsize = AtomicUsize::new(0);
    let s1 = handler.subscribe(|| {
        SHARED_COUNT.fetch_add(1, Ordering::Relaxed);
    });

    let mut exclusive_count = 0;
    handler.subscribe(move || {
        exclusive_count += 1;
        println!("{exclusive_count}")
    });

    handler.spawn().join().unwrap();

    handler.unsubscribe(&s1);

    assert_eq!(SHARED_COUNT.load(Ordering::Relaxed), 2);
    assert_eq!(exclusive_count, 0);
}

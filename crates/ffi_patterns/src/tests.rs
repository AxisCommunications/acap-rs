use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::{spawn, JoinHandle};

mod sys {
    use std::collections::HashMap;
    use std::ffi::c_void;

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
}

struct Deferred(Option<Box<dyn FnOnce()>>);

impl Drop for Deferred {
    fn drop(&mut self) {
        assert!(self.0.is_some());
        self.0.take().unwrap()()
    }
}

impl Deferred {
    unsafe fn new<T: 'static>(ptr: *mut T) -> Self {
        Self(Some(Box::new(move || drop(Box::from_raw(ptr)))))
    }
}

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
}

impl Drop for Handler {
    fn drop(&mut self) {
        unsafe {
            sys::handler_free(self.raw);
        }
    }
}

unsafe impl Send for Handler {}

impl Handler {
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
    fn subscribe<F>(&self, callback: Option<F>) -> Subscription
    where
        F: FnMut() + Send + 'static,
    {
        let raw_callback = callback.map(|c| Box::into_raw(Box::new(c)));
        let callback = raw_callback.map(|c| unsafe { Deferred::new(c) });
        unsafe {
            let handle = sys::handler_subscribe(
                self.raw,
                if raw_callback.is_none() {
                    None
                } else {
                    Some(Self::trampoline::<F>)
                },
                match raw_callback {
                    None => ptr::null_mut(),
                    Some(callback) => callback as *mut c_void,
                },
            );
            let handle = Subscription(handle);
            if let Some(callback) = callback {
                self.callbacks.lock().unwrap().insert(handle, callback);
            }
            handle
        }
    }

    fn unsubscribe(&self, handle: Subscription) {
        self.callbacks.lock().unwrap().remove(&handle);
    }

    unsafe extern "C" fn trampoline<F>(user_data: *mut c_void)
    where
        F: FnMut(),
    {
        let callback = &mut *(user_data as *mut F);
        callback();
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Subscription(i32);

#[test]
fn transfer_callbacks_with_unsubscribe() {
    let handler = Handler::new();

    static SHARED_COUNT: AtomicUsize = AtomicUsize::new(0);
    let s1 = handler.subscribe(Some(|| {
        SHARED_COUNT.fetch_add(1, Ordering::Relaxed);
    }));

    let mut exclusive_count = 0;
    let s2 = handler.subscribe(Some(move || {
        exclusive_count += 1;
        println!("{exclusive_count}")
    }));

    let s3 = handler.subscribe::<fn()>(None);

    handler.spawn().join().unwrap();

    handler.unsubscribe(s2);
    handler.unsubscribe(s1);
    handler.unsubscribe(s3);

    assert_eq!(SHARED_COUNT.load(Ordering::Relaxed), 2);
    assert_eq!(exclusive_count, 0);
}

#[test]
fn transfer_callbacks_without_unsubscribe() {
    let handler = Handler::new();

    static SHARED_COUNT: AtomicUsize = AtomicUsize::new(0);
    handler.subscribe(Some(|| {
        SHARED_COUNT.fetch_add(1, Ordering::Relaxed);
    }));

    let mut exclusive_count = 0;
    handler.subscribe(Some(move || {
        exclusive_count += 1;
        println!("{exclusive_count}")
    }));

    handler.subscribe::<fn()>(None);

    handler.spawn().join().unwrap();

    assert_eq!(SHARED_COUNT.load(Ordering::Relaxed), 2);
    assert_eq!(exclusive_count, 0);
}

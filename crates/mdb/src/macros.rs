macro_rules! suppress_unwind {
    ($f:expr) => {
        ::std::panic::catch_unwind($f).unwrap_or_else(|e| {
            // TODO: Verify that these cannot panic or replace them
            match e.downcast::<::std::string::String>() {
                Ok(e) => ::log::error!("Caught panic in callback (string) {e}"),
                Err(e) => ::log::error!("Caught panic in callback (other) {e:?}"),
            };
        });
    };
}

macro_rules! try_func {
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut ::mdb_sys::mdb_error_t = ::std::ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if !error.is_null() {
            debug_assert!(!success);
            return Err(crate::error::Error::new(error))
        } else {
            debug_assert!(success);
        }
    }}
}

pub(crate) use {suppress_unwind, try_func};

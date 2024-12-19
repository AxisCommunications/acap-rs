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

pub(crate) use suppress_unwind;

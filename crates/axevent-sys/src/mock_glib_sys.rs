use std::sync::Arc;

#[derive(Debug)]
struct _GDateTime {
    seconds_since_epoch: i64,
}

#[derive(Debug)]
pub struct GDateTime(Arc<_GDateTime>);

impl GDateTime {
    pub fn new(seconds_since_epoch: i64) -> Self {
        Self(Arc::new(_GDateTime {
            seconds_since_epoch,
        }))
    }
    pub fn seconds_since_epoch(&self) -> i64 {
        self.0.seconds_since_epoch
    }
}

pub unsafe fn g_date_time_ref(datetime: *mut GDateTime) -> *mut GDateTime {
    Box::into_raw(Box::new(GDateTime(Arc::clone(&(*datetime).0))))
}

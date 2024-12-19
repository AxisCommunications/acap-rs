use std::{marker::PhantomData, slice::from_raw_parts};

pub struct Message<'a> {
    ptr: *const mdb_sys::mdb_message_t,
    _marker: PhantomData<&'a mdb_sys::mdb_message_t>,
}

impl Message<'_> {
    // FIXME: Safety
    pub(crate) unsafe fn from_raw(ptr: *const mdb_sys::mdb_message_t) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
    pub fn payload(&self) -> &[u8] {
        unsafe {
            let payload = *mdb_sys::mdb_message_get_payload(self.ptr);
            from_raw_parts(payload.data, payload.size)
        }
    }

    // TODO: Consider other types.
    // This is a monotonic timestamp but I haven't been able to verify that it is compatible with
    // `Instant` nor that it is even possible to create `Instant`s.
    pub fn timestamp(&self) -> &libc::timespec {
        unsafe {
            mdb_sys::mdb_message_get_timestamp(self.ptr)
                .as_ref()
                .expect("the C API guarantees that the timestamp is not null")
        }
    }
}

pub struct OwnedMessage {
    ptr: *mut mdb_sys::mdb_message_t,
}

impl OwnedMessage {
    pub(crate) fn into_raw(self) -> *mut mdb_sys::mdb_message_t {
        self.ptr as *mut _
    }
}

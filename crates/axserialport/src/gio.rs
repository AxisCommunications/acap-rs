use std::{
    ffi::CStr,
    os::fd::{AsRawFd, BorrowedFd},
    ptr,
};

use glib::{
    translate::{from_glib_full, IntoGlib},
    ControlFlow,
};
use glib_sys::{
    g_io_add_watch, g_io_channel_flush, g_io_channel_read_chars, g_io_channel_set_encoding,
    g_io_channel_unix_new, g_io_channel_write_chars, gboolean, gpointer, GError, GIOChannel,
    GIOCondition, GFALSE, G_IO_STATUS_AGAIN, G_IO_STATUS_EOF, G_IO_STATUS_ERROR,
    G_IO_STATUS_NORMAL,
};

use crate::Result;

macro_rules! try_func {
     ($func:ident, $($arg:expr),* $(,)?) => {{
         let mut error: *mut GError = ptr::null_mut();
         let is_ok = $func($( $arg, )* &mut error);
         debug_assert_ne!(is_ok == glib::ffi::GFALSE, error.is_null());
            if error.is_null() {
                Ok(())
            } else {
                Err(glib::translate::from_glib_full(error))
            }
     }}
 }

macro_rules! try_func_retval {
     ($func:ident, $($arg:expr),+ $(,)?) => {{
         let mut error: *mut GError = ptr::null_mut();
         let retval = $func($( $arg ),+, &mut error);
         if error.is_null() {
             Ok(retval)
         } else {
             Err(glib::translate::from_glib_full(error))
         }
     }}
 }

unsafe extern "C" fn trampoline<F: FnMut(&mut IOChannel, Condition) -> ControlFlow + 'static>(
    channel: *mut GIOChannel,
    condition: GIOCondition,
    user_data: gpointer,
) -> gboolean {
    // TODO: Use something like `Borrowed`.
    let mut channel = IOChannel(channel);
    let condition = Condition::from_int(condition);
    let callback = &mut *(user_data as *mut F);
    callback(&mut channel, condition).into_glib()
}

pub struct IOChannel(*mut glib_sys::GIOChannel);

impl IOChannel {
    // TODO: Propagate lifetime
    pub fn from_borrowed_fd(fd: BorrowedFd) -> Option<Self> {
        unsafe {
            let iochannel = g_io_channel_unix_new(fd.as_raw_fd());
            if iochannel.is_null() {
                None
            } else {
                Some(Self(iochannel))
            }
        }
    }

    pub fn set_encoding(&mut self, encoding: Option<&CStr>) -> Result<()> {
        unsafe {
            try_func!(
                g_io_channel_set_encoding,
                self.0,
                match encoding {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                }
            )
        }
    }

    pub fn watch_local<F>(&mut self, condition: Condition, func: F)
    where
        F: FnMut(&mut IOChannel, Condition) -> ControlFlow + 'static,
    {
        unsafe {
            g_io_add_watch(
                self.0,
                condition as u32,
                Some(trampoline::<F>),
                Box::into_raw(Box::new(func)) as gpointer,
            );
        }
    }

    pub fn read_chars(&mut self, count: usize) -> Result<Vec<u8>> {
        unsafe {
            let mut bytes_read: usize = 0;
            let mut error: *mut GError = ptr::null_mut();
            let mut buf = Vec::with_capacity(count);
            let status = g_io_channel_read_chars(
                self.0,
                buf.as_mut_ptr(),
                count,
                &mut bytes_read,
                &mut error,
            );
            if error.is_null() {
                debug_assert_eq!(status, G_IO_STATUS_NORMAL);
                buf.set_len(bytes_read);
                Ok(buf)
            } else {
                debug_assert_eq!(status, G_IO_STATUS_ERROR);
                Err(from_glib_full(error))
            }
        }
    }

    pub fn write_chars(&mut self, data: &[u8]) -> Result<usize> {
        unsafe {
            let mut bytes_written: usize = 0;
            let mut error: *mut GError = ptr::null_mut();
            let status = g_io_channel_write_chars(
                self.0,
                data.as_ptr(),
                data.len() as isize,
                &mut bytes_written,
                &mut error,
            );
            if error.is_null() {
                debug_assert_eq!(status, G_IO_STATUS_NORMAL);
                Ok(bytes_written)
            } else {
                debug_assert_eq!(status, G_IO_STATUS_ERROR);
                Err(from_glib_full(error))
            }
        }
    }

    pub fn flush(&mut self) -> Result<Status> {
        unsafe {
            // FIXME: Return Status
            match try_func_retval!(g_io_channel_flush, self.0) {
                Ok(s) => Ok(Status::from_int(s)),
                Err(e) => Err(e),
            }
        }
    }
}

impl Drop for IOChannel {
    fn drop(&mut self) {
        unsafe {
            glib_sys::g_io_channel_shutdown(self.0, GFALSE, std::ptr::null_mut());
            glib_sys::g_io_channel_unref(self.0);
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Condition {
    In = glib_sys::G_IO_IN,
    Out = glib_sys::G_IO_OUT,
    Pri = glib_sys::G_IO_PRI,
    Err = glib_sys::G_IO_ERR,
    Hup = glib_sys::G_IO_HUP,
    Nval = glib_sys::G_IO_NVAL,
}

impl Condition {
    fn from_int(value: GIOCondition) -> Self {
        match value {
            glib_sys::G_IO_IN => Condition::In,
            glib_sys::G_IO_OUT => Condition::Out,
            glib_sys::G_IO_PRI => Condition::Pri,
            glib_sys::G_IO_ERR => Condition::Err,
            glib_sys::G_IO_HUP => Condition::Hup,
            glib_sys::G_IO_NVAL => Condition::Nval,
            _ => panic!(),
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Status {
    Error = G_IO_STATUS_ERROR,
    Normal = G_IO_STATUS_NORMAL,
    Eof = G_IO_STATUS_EOF,
    Again = G_IO_STATUS_AGAIN,
}

impl Status {
    fn from_int(value: i32) -> Self {
        match value {
            G_IO_STATUS_ERROR => Status::Error,
            G_IO_STATUS_NORMAL => Status::Normal,
            G_IO_STATUS_EOF => Status::Eof,
            G_IO_STATUS_AGAIN => Status::Again,
            _ => panic!(),
        }
    }
}

//! Bindings for the [Serial Port API](https://developer.axis.com/acap/api/src/api/axserialport/html/index.html).
pub mod gio;

use std::{os::fd::BorrowedFd, ptr};

use axserialport_sys::{
    ax_serial_cleanup, ax_serial_get_fd, ax_serial_init, ax_serial_port_enable,
    ax_serial_set_baudrate, ax_serial_set_bias, ax_serial_set_databits, ax_serial_set_parity,
    ax_serial_set_portmode, ax_serial_set_stopbits, ax_serial_set_termination,
    ax_serial_sync_port_settings, AXSerialConfig,
};
pub use axserialport_sys::{
    AXSerialBaudrate as BaudRate, AXSerialDatabits as DataBits, AXSerialEnable as Enable,
    AXSerialParity as Parity, AXSerialPortmode as PortMode, AXSerialStopbits as StopBits,
};
use glib::translate::from_glib_full;
pub use glib::Error;
use glib_sys::{gboolean, GError};

type Result<T> = core::result::Result<T, Error>;

unsafe fn try_into_unit(is_ok: gboolean, error: *mut GError) -> Result<()> {
    debug_assert_ne!(is_ok == glib::ffi::GFALSE, error.is_null());
    if error.is_null() {
        Ok(())
    } else {
        Err(from_glib_full(error))
    }
}

macro_rules! try_func {
     ($func:ident, $($arg:expr),* $(,)?) => {{
         let mut error: *mut GError = ptr::null_mut();
         let success = $func($( $arg, )* &mut error);
         try_into_unit(success, error)
     }}
 }

macro_rules! try_func_retval {
     ($func:ident, $($arg:expr),+ $(,)?) => {{
         let mut error: *mut GError = ptr::null_mut();
         let retval = $func($( $arg ),+, &mut error);
         if error.is_null() {
             Ok(retval)
         } else {
             Err(from_glib_full(error))
         }
     }}
 }

pub struct Config(*mut AXSerialConfig);

impl Config {
    pub fn try_new(port_index: u32) -> Result<Self> {
        unsafe { try_func_retval!(ax_serial_init, port_index).map(Self) }
    }

    pub fn baudrate(&mut self, baudrate: BaudRate) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_baudrate, self.0, baudrate) }.map(|()| self)
    }

    pub fn bias(&mut self, enable: Enable) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_bias, self.0, enable) }.map(|()| self)
    }

    pub fn databits(&mut self, databits: DataBits) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_databits, self.0, databits) }.map(|()| self)
    }

    pub fn parity(&mut self, parity: Parity) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_parity, self.0, parity) }.map(|()| self)
    }

    pub fn portmode(&mut self, portmode: PortMode) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_portmode, self.0, portmode) }.map(|()| self)
    }

    pub fn port_enable(&mut self, enable: Enable) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_port_enable, self.0, enable) }.map(|()| self)
    }

    pub fn stopbits(&mut self, stopbits: StopBits) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_stopbits, self.0, stopbits) }.map(|()| self)
    }

    pub fn termination(&mut self, enable: Enable) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_set_termination, self.0, enable) }.map(|()| self)
    }

    pub fn sync(&mut self) -> Result<&mut Self> {
        unsafe { try_func!(ax_serial_sync_port_settings, self.0) }.map(|()| self)
    }

    // TODO: Proper lifetime
    pub fn get_fd(&mut self) -> Result<BorrowedFd<'static>> {
        unsafe { try_func_retval!(ax_serial_get_fd, self.0).map(|fd| BorrowedFd::borrow_raw(fd)) }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe { ax_serial_cleanup(self.0) };
    }
}

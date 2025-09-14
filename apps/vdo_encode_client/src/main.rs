//! This application is a basic VDO type of application.
//!
//! The application starts a VDO stream and illustrates how to continuously capture frames from the
//! VDO service, access the received buffer contents, as well as the frame metadata.
//!
//! # Arguments
//!
//! - `format`: A string describing the video compression format.
//!   Possible values are `h264` (default), `h265`, `jpeg`, `nv12`, and `y800`.
//! - `frames`: An integer specifying the number of captured frames.
//! - `output`: The output filename.
//!
use log::info;

fn main() {
    acap_logging::init_logger();
    unsafe { assert!(!vdo_sys::vdo_map_new().is_null()) };
    info!("vdo map created");
    todo!("Implement the real example")
}

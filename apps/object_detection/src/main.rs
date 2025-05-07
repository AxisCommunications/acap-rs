//! This application loads a larod model which takes an image as input and outputs values
//! corresponding to the class, score and location of detected objects in the image.
//!
//! # Arguments
//!
//! 1. `MODEL`: a string describing the path to the model.
//! 2. `WIDTH`: an integer for the input width. ß
//! 3. `HEIGHT`: an integer for the input height.
//! 4. `QUALITY`: an integer for the desired jpeg quality.
//! 5. `RAW_WIDTH`: an integer for camera width resolution.
//! 6. `RAW_HEIGHT`: an integer for camera height resolution.
//! 7. `THRESHOLD`: an integer ranging from 0 to 100 to select good detections.
//! 8. `LABELSFILE`: a string describing the path to the label txt.

use log::{error, info};

fn main() {
    acap_logging::init_logger();
    let mut conn: *mut larod_sys::larodConnection = std::ptr::null_mut();
    let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
    if unsafe{!larod_sys::larodConnect(&mut conn, &mut error)} {
        error!("Could not connect to larod");
        return;
    }
    assert!(error.is_null());

    let mut num_sessions = u64::MAX; 
    if unsafe{!larod_sys::larodGetNumSessions(conn, &mut num_sessions, &mut error)} {
        error!("Could not get the number of sessions");
        return;
    }

    info!("Number of sessions: {num_sessions}");
    todo!("Implement the real example")
}

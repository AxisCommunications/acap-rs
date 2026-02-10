//! VDO Example Application
//!
//! This application demonstrates the VDO (Video Capture) API by capturing
//! frames from the camera in various formats.
//!
//! It tests:
//! - Stream creation with different formats (YUV, JPEG, H.264)
//! - Frame capture and metadata access
//! - Proper resource cleanup

// These format tests run on the device as an ACAP application rather than as
// unit tests because they require access to actual camera hardware via the VDO API.

use log::{error, info};
use vdo::{Error, Resolution, Stream, VdoFormat};

fn capture_format(name: &str, format: VdoFormat, num_frames: usize) -> Result<(), Error> {
    info!("=== Testing {} format ===", name);

    let stream = Stream::builder()
        .channel(0)
        .format(format)
        .resolution(Resolution::Exact {
            width: 640,
            height: 480,
        })
        .framerate(15)
        .build()?;

    info!("{}: Stream created successfully", name);

    // Get stream info
    if let Ok(stream_info) = stream.info() {
        info!("{}: Stream info:", name);
        stream_info.dump();
    }

    let running = stream.start()?;
    info!("{}: Stream started", name);

    for i in 0..num_frames {
        let buffer = running.next_buffer()?;
        let size = buffer.size();
        let seq = buffer.sequence_number();
        let ts = buffer.timestamp();

        info!(
            "{}: Frame {}: {} bytes, seq={}, timestamp={}us",
            name, i, size, seq, ts
        );

        // For JPEG, verify magic bytes
        if format == VdoFormat::VDO_FORMAT_JPEG {
            let data = buffer.as_slice()?;
            if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
                info!("{}: Frame {} has valid JPEG header", name, i);
            } else {
                error!("{}: Frame {} has INVALID JPEG header!", name, i);
            }
        }
    }

    running.stop();
    info!("{}: Stream stopped successfully", name);
    info!("");

    Ok(())
}

fn main() {
    acap_logging::init_logger();

    info!("VDO Example Application starting...");
    info!("Testing VDO safe Rust bindings");
    info!("");

    // Test YUV (most portable format)
    match capture_format("YUV", VdoFormat::VDO_FORMAT_YUV, 5) {
        Ok(()) => info!("YUV test: PASSED"),
        Err(e) => error!("YUV test: FAILED - {}", e),
    }

    // Test JPEG
    match capture_format("JPEG", VdoFormat::VDO_FORMAT_JPEG, 5) {
        Ok(()) => info!("JPEG test: PASSED"),
        Err(e) => error!("JPEG test: FAILED - {}", e),
    }

    // Test H.264
    match capture_format("H.264", VdoFormat::VDO_FORMAT_H264, 10) {
        Ok(()) => info!("H.264 test: PASSED"),
        Err(e) => error!("H.264 test: FAILED - {}", e),
    }

    // Test H.265 (might not be supported on all platforms)
    match capture_format("H.265", VdoFormat::VDO_FORMAT_H265, 5) {
        Ok(()) => info!("H.265 test: PASSED"),
        Err(e) => {
            if let Error::Vdo(ref vdo_err) = e {
                if vdo_err.code_name() == "VDO_ERROR_NOT_SUPPORTED" {
                    info!("H.265 test: SKIPPED (not supported on this platform)");
                } else {
                    error!("H.265 test: FAILED - {}", e);
                }
            } else {
                error!("H.265 test: FAILED - {}", e);
            }
        }
    }

    info!("");
    info!("VDO Example Application completed!");
}

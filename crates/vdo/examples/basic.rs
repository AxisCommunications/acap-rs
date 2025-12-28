//! Basic example of using VDO to capture video frames.
//!
//! This example creates a video stream, captures a few frames, and prints
//! information about each frame.

use vdo::{Stream, VdoFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    env_logger::init();

    println!("Creating video stream...");

    // Create a stream with YUV format (most portable across platforms)
    let mut stream = Stream::builder()
        .channel(0)
        .format(VdoFormat::VDO_FORMAT_YUV)
        .resolution(640, 480)
        .framerate(15)
        .build()?;

    println!("Starting stream...");
    let mut running = stream.start()?;

    println!("Capturing frames...");
    for (i, buffer) in running.iter().take(10).enumerate() {
        let frame = buffer.frame()?;
        println!(
            "Frame {}: {} bytes, seq={}, timestamp={}us",
            i,
            frame.size(),
            frame.sequence_number(),
            frame.timestamp()
        );
    }

    println!("Stopping stream...");
    running.stop()?;

    println!("Done!");
    Ok(())
}

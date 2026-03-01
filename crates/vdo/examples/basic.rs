//! Basic example of using VDO to capture video frames.
//!
//! This example creates a video stream, captures a few frames, and prints
//! information about each frame.

use vdo::{Resolution, Stream, VdoFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    env_logger::init();

    println!("Creating video stream...");

    // Create a stream with YUV format (most portable across platforms)
    let stream = Stream::builder()
        .channel(0)
        .format(VdoFormat::VDO_FORMAT_YUV)
        .resolution(Resolution::Exact {
            width: 640,
            height: 480,
        })
        .framerate(15)
        .build()?;

    println!("Starting stream...");
    let running = stream.start()?;

    println!("Capturing frames...");
    for i in 0..10 {
        let buffer = running.next_buffer()?;
        println!(
            "Frame {}: {} bytes, seq={}, timestamp={}us",
            i,
            buffer.size(),
            buffer.sequence_number(),
            buffer.timestamp()
        );
    }

    println!("Stopping stream...");
    drop(running);

    println!("Done!");
    Ok(())
}

use anyhow::{Error, Result};
use vdo::{Stream, VdoBufferStrategy, VdoFormat};

fn main() -> Result<()> {
    let mut stream = Stream::builder()
        .channel(0)
        .format(VdoFormat::VDO_FORMAT_PLANAR_RGB)
        .width(1920)
        .height(1080)
        .buffer_strategy(VdoBufferStrategy::VDO_BUFFER_STRATEGY_EXPLICIT)
        .build()
        .expect("Unable to create stream");
    stream
        .allocate_buffers(5)
        .expect("failed to allocate buffers");
    let mut running_stream = stream.start().expect("failed to start stream");
    for (idx, buffer) in running_stream.iter().enumerate() {
        let frame = buffer.frame().expect("failed to get frame from buffer");
        println!("frame size: {}", frame.size());
        if idx > 5 {
            break;
        }
    }
    Ok(())
}

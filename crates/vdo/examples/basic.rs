use anyhow::{Error, Result};
use vdo::Stream;

fn main() -> Result<()> {
    let stream = Stream::builder()
        .with_channel()
        .with_format()
        .with_width()
        .with_height()
        .build();
    stream.start();
    while let Ok(frame) = stream.next_frame() {}
    Ok(())
}

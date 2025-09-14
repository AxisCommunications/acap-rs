#![forbid(unsafe_code)]
//! An example of how to draw bounding boxes using the Bounding Box API.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use bbox::flex::{Bbox, Color};
use log::info;

fn example_single_channel(running: Arc<AtomicBool>) -> anyhow::Result<()> {
    let mut bbox = Bbox::try_view_new(1)?;
    bbox.try_clear().unwrap();

    let red = Color::from_rgb(0xff, 0, 0);
    let blue = Color::from_rgb(0, 0, 0xff);
    let green = Color::from_rgb(0, 0xff, 0);

    bbox.try_style_outline()?; // Switch to outline style
    bbox.try_thickness_thin()?; // Switch to thin lines
    bbox.try_color(red)?; // Switch to red [This operation is fast!]
    bbox.try_rectangle(0.05, 0.05, 0.95, 0.95)?; // Draw a thin red outline rectangle

    bbox.try_style_corners()?; // Switch to corners style
    bbox.try_thickness_thick()?; // Switch to thick lines
    bbox.try_color(blue)?; // Switch to blue [This operation is fast!]
    bbox.try_rectangle(0.40, 0.40, 0.60, 0.60)?; // Draw thick blue corners

    bbox.try_style_corners()?; // Switch to corners style
    bbox.try_thickness_medium()?; // Switch to medium lines
    bbox.try_color(blue)?; // Switch to blue [This operation is fast!]
    bbox.try_rectangle(0.30, 0.30, 0.50, 0.50)?; // Draw medium blue corners

    bbox.try_style_outline()?; // Switch to outline style
    bbox.try_thickness_thin()?; // Switch to thin lines
    bbox.try_color(red)?; // Switch to red [This operation is fast!]

    // Draw a thin red quadrilateral
    bbox.try_quad(0.10, 0.10, 0.30, 0.12, 0.28, 0.28, 0.11, 0.30)?;

    // Draw a green polyline
    bbox.try_color(green)?; // Switch to green [This operation is fast!]
    bbox.try_move_to(0.2, 0.2)?;
    bbox.try_line_to(0.5, 0.5)?;
    bbox.try_line_to(0.8, 0.4)?;
    bbox.try_draw_path()?;

    bbox.try_commit(0)?;
    if running.load(Ordering::SeqCst) {
        sleep(Duration::from_secs(5));
    }
    Ok(())
}

fn example_multiple_channels(running: Arc<AtomicBool>) -> anyhow::Result<()> {
    // Draw on channel 1 and 2
    let mut bbox = Bbox::try_new(&[1, 2])?;

    // If camera lacks video output, this call will succeed but not do anything.
    bbox.try_video_output(true)?;

    // Create all needed colors [These operations are slow!]
    let colors = [
        Color::from_rgb(0xff, 0, 0),
        Color::from_rgb(0, 0xff, 0),
        Color::from_rgb(0, 0, 0xff),
    ];

    // Switch to thick corner style
    bbox.try_thickness_thick()?;
    bbox.try_style_corners()?;

    let w = 1920.0;
    let h = 1080.0;
    let box_w = 100.0 / w;
    let box_h = 100.0 / h;

    for i in 0..32 {
        let x = 200.0 * (i % 8) as f32 / w;
        let y = 200.0 * (i / 8) as f32 / h;

        // Switch color [This operation is fast!]
        bbox.try_color(colors[i % colors.len()]).unwrap();

        bbox.try_rectangle(x, y, x + box_w, y + box_h).unwrap();
    }

    bbox.try_commit(0)?;
    if running.load(Ordering::SeqCst) {
        sleep(Duration::from_secs(5));
    }
    Ok(())
}

fn example_clear(running: Arc<AtomicBool>) -> anyhow::Result<()> {
    let mut bbox = Bbox::try_new(&[1])?;
    bbox.try_clear()?;
    bbox.try_commit(0)?;
    if running.load(Ordering::SeqCst) {
        sleep(Duration::from_secs(5));
    }
    Ok(())
}

fn main() {
    acap_logging::init_logger();
    // Even though running is not used for signal handling yet, it is useful for removing the sleep
    // in the smoke tests.
    // TODO: Consider implementing signal handling.
    // Even though all bbox functions return `std::io::Result`, not all are equally severe;
    // while e.g. `try_color` may succeed after failing once, this is unlikely to be the case for
    // e.g. `try_video_output`.
    // TODO: Consider implementing error handling that is closer to that of the C example.
    let running = Arc::new(AtomicBool::new(true));
    for i in 0.. {
        example_single_channel(Arc::clone(&running)).unwrap();
        example_multiple_channels(Arc::clone(&running)).unwrap();
        example_clear(Arc::clone(&running)).unwrap();
        if i == 0 {
            info!("All examples succeeded.")
        }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicBool, Arc};

    use super::*;

    #[test]
    fn smoke_test() {
        let running = Arc::new(AtomicBool::new(false));
        example_single_channel(Arc::clone(&running)).unwrap();
        example_multiple_channels(Arc::clone(&running)).unwrap();
        example_clear(Arc::clone(&running)).unwrap();
    }
}

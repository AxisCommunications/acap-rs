//! An example of how to draw bounding boxes using the Bounding Box API.

use std::{thread::sleep, time::Duration};

use bbox::flex::{Bbox, Color};

fn example_single_channel() -> anyhow::Result<()> {
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
    sleep(Duration::from_secs(5));
    Ok(())
}
fn main() {
    acap_logging::init_logger();
    example_single_channel().unwrap();
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use crate::example_single_channel;

    #[test]
    fn smoke_test() {
        example_single_channel().unwrap();
    }
}

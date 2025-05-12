// #![forbid(unsafe_code)]

use std::{
    ffi::{c_float, c_int},
    sync::Mutex,
};

use anyhow::{bail, Context};
use axoverlay::{
    redraw, set_palette_color, Backend, Camera, CleanupGuard, Color, OverlayData, OverlayId,
    PosType, Settings, StreamData,
};
use libc::{SIGINT, SIGTERM};
use log::{error, info};

// TODO: Investiagate if this can be thread local
static SHARED_STATE: Mutex<Option<GlobalState>> = Mutex::new(None);
const PALETTE_VALUE_RANGE: f64 = 255.0;
struct GlobalState {
    animation_timer: glib::SourceId,
    overlay_id: OverlayId,
    overlay_id_text: OverlayId,
    counter: usize,
    top_color: i32,
    bottom_color: i32,
}

fn index2cairo(color_index: i32) -> f64 {
    ((color_index << 4) + color_index) as f64 / PALETTE_VALUE_RANGE
}

fn draw_rectangle(
    context: &cairo::Context,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    color_index: i32,
    line_width: f64,
) {
    let val = index2cairo(color_index);
    context.set_source_rgba(val, val, val, val);
    context.set_operator(cairo::Operator::Source);
    context.set_line_width(line_width);
    context.rectangle(left, top, right - left, bottom - top);
    let _ = context.stroke();
}

fn draw_text(context: &cairo::Context, pos_x: f64, pos_y: f64, counter: usize) {
    //  Show text in black
    context.set_source_rgb(0.0, 0.0, 0.0);
    context.select_font_face("serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    context.set_font_size(32.0);

    // Position the text at a fix centered position
    let str_length = "Countdown  ";
    let Ok(te_length) = context.text_extents(str_length) else {
        return;
    };
    context.move_to(pos_x - te_length.width() / 2.0, pos_y);

    // Add the counter number to the shown text
    let s = format!("Countdown {}", counter);
    let _ = context.text_extents(&s);
    let _ = context.show_text(&s);
}

fn adjustment_cb(
    _id: OverlayId,
    stream: &StreamData,
    _postype: &PosType,
    _overlay_x: &mut c_float,
    _overlay_y: &mut c_float,
    overlay_width: &mut c_int,
    overlay_height: &mut c_int,
) {
    *overlay_width = stream.width();
    *overlay_height = stream.height();
    if stream.rotation() == 90 || stream.rotation() == 270 {
        *overlay_width = stream.height();
        *overlay_height = stream.width();
    }

    info!(
        "Stream or rotation changed, overlay resolution is now: {} x {}",
        overlay_width, overlay_height
    );
    info!(
        "Stream or rotation changed, stream resolution is now: {} x {}",
        stream.width(),
        stream.height()
    );
    info!(
        "Stream or rotation changed, rotation angle is now: {}",
        stream.rotation()
    );
}

fn render_overlay_cb(
    rendering_context: &cairo::Context,
    id: OverlayId,
    stream: &StreamData,
    _postype: PosType,
    _overlay_x: c_float,
    _overlay_y: c_float,
    overlay_width: c_int,
    overlay_height: c_int,
) {
    info!("Render callback for camera: {}", stream.camera());
    info!(
        "Render callback for overlay: {} x {}",
        overlay_width, overlay_height
    );
    info!(
        "Render callback for stream: {} x {}",
        stream.width(),
        stream.height()
    );
    info!("Render callback for rotation: {}", stream.rotation());

    let state = SHARED_STATE.lock().unwrap();
    let GlobalState {
        overlay_id,
        overlay_id_text,
        counter,
        top_color,
        bottom_color,
        ..
    } = state.as_ref().unwrap();

    if id == *overlay_id {
        let val = index2cairo(0);
        rendering_context.set_source_rgba(val, val, val, val);
        rendering_context.set_operator(cairo::Operator::Source);
        rendering_context.rectangle(0.0, 0.0, overlay_width as f64, overlay_height as f64);
        let _ = rendering_context.fill();

        //  Draw a top rectangle in toggling color
        draw_rectangle(
            rendering_context,
            0.0,
            0.0,
            overlay_width as f64,
            overlay_height as f64 / 4.0,
            *top_color,
            9.6,
        );

        //  Draw a bottom rectangle in toggling color
        draw_rectangle(
            rendering_context,
            0.0,
            overlay_height as f64 * 3.0 / 4.0,
            overlay_width as f64,
            overlay_height as f64,
            *bottom_color,
            2.0,
        );
    } else if id == *overlay_id_text {
        //  Show text in black
        draw_text(
            rendering_context,
            overlay_width as f64 / 2.0,
            overlay_height as f64 / 2.0,
            *counter,
        );
    } else {
        info!("Unknown overlay id!");
    }
}

fn update_overlay_cb() -> glib::ControlFlow {
    if let Some(GlobalState {
        counter,
        top_color,
        bottom_color,
        ..
    }) = SHARED_STATE.lock().unwrap().as_mut()
    {
        *counter = if *counter < 1 { 10 } else { *counter - 1 };
        *top_color = if *top_color > 2 { 1 } else { *top_color + 1 };
        *bottom_color = if *bottom_color > 2 {
            1
        } else {
            *bottom_color + 1
        };
    }

    if let Err(e) = redraw() {
        error!("Failed to redraw overlay: {e:?}");
    }

    glib::ControlFlow::Continue
}

fn main() -> anyhow::Result<()> {
    acap_logging::init_logger();
    let main_loop = glib::MainLoop::new(None, false);
    glib::unix_signal_add_once(SIGTERM, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });
    glib::unix_signal_add_once(SIGINT, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });

    if !Backend::CairoImage.is_supported() {
        bail!("AXOVERLAY_CAIRO_IMAGE_BACKEND is not supported");
    }

    Settings::default()
        .render_callback(render_overlay_cb)
        .adjustment_callback(adjustment_cb)
        .backend(Backend::CairoImage)
        .init()
        .context("Failed to initialize axoverlay")?;

    Ok(())
        .and_then(|()| set_palette_color(0, &mut Color::new(0, 0, 0, 0, false)))
        .and_then(|()| set_palette_color(1, &mut Color::new(255, 0, 0, 255, false)))
        .and_then(|()| set_palette_color(2, &mut Color::new(0, 255, 0, 255, false)))
        .and_then(|()| set_palette_color(3, &mut Color::new(0, 0, 255, 255, false)))
        .context("Failed to set palette color")?;

    let camera = Camera::new(1);
    let camera_width = camera
        .max_width()
        .context("Failed to get max resolution width")?;
    let camera_height = camera
        .max_height()
        .context("Failed to get max resolution height")?;
    info!("Max resolution (width x height): {camera_width} x {camera_height}");

    let overlay_id = OverlayData::default()
        .width(camera_width)
        .height(camera_height)
        .colorspace(axoverlay::ColorSpace::ARGB32)
        .create_overlay()
        .context("Failed to create first overlay")?;

    let overlay_id_text = OverlayData::default()
        .width(camera_width)
        .height(camera_height)
        .colorspace(axoverlay::ColorSpace::ARGB32)
        .create_overlay()
        .context("Failed to create first overlay")?;

    let _cleanup_guard = CleanupGuard::default();

    redraw().context("Failed to draw overlays")?;

    let animation_timer = glib::timeout_add_seconds(1, update_overlay_cb);

    SHARED_STATE.lock().unwrap().replace(GlobalState {
        animation_timer,
        overlay_id,
        overlay_id_text,
        counter: 10,
        top_color: 1,
        bottom_color: 3,
    });

    main_loop.run();

    SHARED_STATE
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .animation_timer
        .remove();

    Ok(())
}

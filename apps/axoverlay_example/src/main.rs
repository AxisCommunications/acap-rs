#![forbid(unsafe_code)]

use std::cell::RefCell;

use anyhow::{bail, Context};
use axoverlay::{redraw, AnchorPoint, Backend, OverlayInfo, PositionType, Settings, StreamData};
use libc::{SIGINT, SIGTERM};
use log::{error, info, warn};

const PALETTE_VALUE_RANGE: f64 = 255.0;

thread_local! {
    static COUNTER: RefCell<i32> = const { RefCell::new(10) };
    static TOP_COLOR: RefCell<i32> = const { RefCell::new(1) };
    static BOTTOM_COLOR: RefCell<i32> = const { RefCell::new(3) };
    static OVERLAY_ID: RefCell<Option<i32>> = const { RefCell::new(None) };
    static OVERLAY_ID_TEXT: RefCell<Option<i32>> = const { RefCell::new(None) };
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
    if let Err(e) = context.stroke() {
        warn!("Error when drawing rectangle: {e:?}");
    }
}

fn draw_text(context: &cairo::Context, pos_x: f64, pos_y: f64) -> anyhow::Result<()> {
    //  Show text in black
    context.set_source_rgb(0.0, 0.0, 0.0);
    context.select_font_face("serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    context.set_font_size(32.0);

    // Position the text at a fix-centered position
    let str_length = "Countdown  ";
    let te_length = context.text_extents(str_length)?;
    context.move_to(pos_x - te_length.width() / 2.0, pos_y);

    // Add the counter-number to the shown text
    let s = COUNTER.with(|c| format!("Countdown {}", *c.borrow()));
    let _te = context.text_extents(&s)?;
    context.show_text(&s)?;
    Ok(())
}

fn adjustment_cb(
    _id: i32,
    stream: &StreamData,
    _postype: &PositionType,
    _overlay_x: &mut f32,
    _overlay_y: &mut f32,
    overlay_width: &mut i32,
    overlay_height: &mut i32,
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
        "Stream or rotation changed, the rotation angle is now: {}",
        stream.rotation()
    );
}

fn render_overlay_cb(
    rendering_context: &cairo::Context,
    id: i32,
    stream: &StreamData,
    _: PositionType,
    info: OverlayInfo,
) {
    let OverlayInfo {
        width: overlay_width,
        height: overlay_height,
        ..
    } = info;
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

    // Unwrap is OK because these variables are set on the same thread before redraw is first called.
    let overlay_id = OVERLAY_ID.with(|id| *id.borrow()).unwrap();
    let overlay_text_id = OVERLAY_ID_TEXT.with(|id| *id.borrow()).unwrap();

    if id == overlay_id {
        let val = index2cairo(0);
        rendering_context.set_source_rgba(val, val, val, val);
        rendering_context.set_operator(cairo::Operator::Source);
        rendering_context.rectangle(0.0, 0.0, overlay_width as f64, overlay_height as f64);
        if let Err(e) = rendering_context.fill() {
            warn!("Failed to render overlay: {e:?}");
        };

        //  Draw a top rectangle in toggling color
        draw_rectangle(
            rendering_context,
            0.0,
            0.0,
            overlay_width as f64,
            overlay_height as f64 / 4.0,
            TOP_COLOR.with_borrow(|c| *c),
            9.6,
        );

        //  Draw a bottom rectangle in toggling color
        draw_rectangle(
            rendering_context,
            0.0,
            overlay_height as f64 * 3.0 / 4.0,
            overlay_width as f64,
            overlay_height as f64,
            BOTTOM_COLOR.with_borrow(|c| *c),
            2.0,
        );
    } else if id == overlay_text_id {
        //  Show text in black
        if let Err(e) = draw_text(
            rendering_context,
            overlay_width as f64 / 2.0,
            overlay_height as f64 / 2.0,
        ) {
            error!("Failed to draw text: {e:?}");
        }
    } else {
        info!("Unknown overlay id!");
    }
}

fn update_overlay_cb() -> glib::ControlFlow {
    let counter = COUNTER.with_borrow_mut(|counter| {
        *counter = if *counter < 1 { 10 } else { *counter - 1 };
        *counter
    });

    if counter == 0 {
        TOP_COLOR.with_borrow_mut(|color| *color = if *color > 2 { 1 } else { *color + 1 });
        BOTTOM_COLOR.with_borrow_mut(|color| *color = if *color > 2 { 1 } else { *color + 1 });
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
        bail!("The cairo backend is not supported");
    }

    let api = Settings::default()
        .render_callback(render_overlay_cb)
        .adjustment_callback(adjustment_cb)
        .backend(Backend::CairoImage)
        .init(&main_loop)
        .context("Failed to initialize axoverlay")?;

    Ok(())
        .and_then(|()| api.color(0, 0, 0, 0, false).set_palette(0))
        .and_then(|()| api.color(255, 0, 0, 255, false).set_palette(1))
        .and_then(|()| api.color(0, 255, 0, 255, false).set_palette(2))
        .and_then(|()| api.color(0, 0, 255, 255, false).set_palette(3))
        .context("Failed to set palette color")?;

    let camera = api.camera(1);
    let camera_width = camera
        .max_width()
        .context("Failed to get max resolution width")?;
    let camera_height = camera
        .max_height()
        .context("Failed to get max resolution height")?;
    info!("Max resolution (width x height): {camera_width} x {camera_height}");

    let rectangle = api
        .overlay_builder()
        .position_type(PositionType::CustomNormalized)
        .anchor_point(AnchorPoint::Center)
        .x(0.0)
        .y(0.0)
        .width(camera_width)
        .height(camera_height)
        .colorspace(axoverlay::ColorSpace::FourBitPalette)
        .create_overlay()
        .context("Failed to create a rectangle overlay")?;
    OVERLAY_ID.with_borrow_mut(|id| *id = Some(rectangle.id()));

    let text = api
        .overlay_builder()
        .position_type(PositionType::CustomNormalized)
        .anchor_point(AnchorPoint::Center)
        .x(0.0)
        .y(0.0)
        .scale_to_stream(false)
        .width(camera_width)
        .height(camera_height)
        .colorspace(axoverlay::ColorSpace::ARGB32)
        .create_overlay()
        .context("Failed to create text overlay")?;
    OVERLAY_ID_TEXT.with_borrow_mut(|id| *id = Some(text.id()));

    redraw().context("Failed to draw overlays")?;

    let animation_timer = glib::timeout_add_seconds(1, update_overlay_cb);

    main_loop.run();

    animation_timer.remove();

    Ok(())
}

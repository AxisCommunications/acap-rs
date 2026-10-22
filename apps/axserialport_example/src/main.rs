#![forbid(unsafe_code)]

use std::time::Instant;

use anyhow::Context;
use axserialport::{
    gio::{Condition, IOChannel},
    BaudRate, Config, DataBits, Enable, Parity, PortMode, StopBits,
};
use glib::MainLoop;
use libc::{SIGINT, SIGTERM};
use log::{error, info};

fn incoming_data(channel: &mut IOChannel, _condition: Condition) -> glib::ControlFlow {
    match channel.read_chars(2) {
        Ok(timestamp) => {
            let min = timestamp[0];
            let sec = timestamp[1];
            info!("incoming_data() timestamp: {min:02}:{sec:02}");
        }
        Err(e) => {
            error!("incoming_data() read error: {}", e.message());
        }
    }
    glib::ControlFlow::Continue
}

const TIMER_INTERVAL_SECS: u64 = 10;

fn send_timer_data(channel: &mut IOChannel, timer: Instant) -> glib::ControlFlow {
    let interval = TIMER_INTERVAL_SECS as f64;
    let elapsed = ((timer.elapsed().as_secs_f64() / interval).round() as u64) * TIMER_INTERVAL_SECS;
    let minutes = elapsed / 60;
    let seconds = elapsed % 60;

    match channel.write_chars(&[minutes as u8, seconds as u8]) {
        Ok(bytes_written) => {
            match channel.flush() {
                Ok(status) => {
                    info!("send_timer_data() wrote {bytes_written} bytes, status: {status:?}");
                }
                Err(e) => {
                    error!("send_timer_data() flush error: {}", e.message());
                }
            }
        }
        Err(e) => {
            error!("send_timer_data() write error: {}", e.message());
        }
    }
    glib::ControlFlow::Continue
}

fn main() -> anyhow::Result<()> {
    acap_logging::init_logger();

    let main_loop = MainLoop::new(None, false);
    glib::unix_signal_add_once(SIGTERM, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });
    glib::unix_signal_add_once(SIGINT, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });

    info!("Starting AxSerialPort application");

    // Config example (product dependent) see product datasheet.
    let mut config = Config::try_new(0)?;
    config
        .port_enable(Enable::Enable)?
        .baudrate(BaudRate::B19200)?
        .bias(Enable::Disable)?
        .databits(DataBits::Eight)?
        .parity(Parity::None)?
        .portmode(PortMode::Rs485Four)?
        .stopbits(StopBits::One)?
        .termination(Enable::Disable)?
        .sync()?;

    let fd = config.get_fd()?;

    let mut iochannel = IOChannel::from_borrowed_fd(fd).context("Failed to get channel")?;
    iochannel.set_encoding(None)?;

    let timer = Instant::now();

    // Add a watch that waits for incoming data, then calls 'incoming_data()'
    // when the conditions are met.
    iochannel.watch_local(Condition::In, incoming_data);

    // Periodically call 'send_timer_data()' every 10 seconds.
    glib::timeout_add_seconds_local(TIMER_INTERVAL_SECS as u32, move || {
        send_timer_data(&mut iochannel, timer)
    });

    main_loop.run();

    info!("Finish AXSerialPort application");
    Ok(())
}

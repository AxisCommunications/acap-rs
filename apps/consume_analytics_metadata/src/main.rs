use std::{ffi::CStr, process::abort, thread::sleep, time::Duration};

use log::{error, info};
use mdb::{Connection, Subscriber, SubscriberConfig};

const TOPIC: &CStr = c"com.axis.analytics_scene_description.v0.beta";
const SOURCE: &CStr = c"1";

fn main() {
    acap_logging::init_logger();

    let connection = Connection::try_new(Some(Box::new(|e| {
        error!("Not connected because {e:?}");
        abort();
    })))
    .unwrap();

    let config = SubscriberConfig::try_new(
        TOPIC,
        SOURCE,
        Box::new(|message| {
            let payload = String::from_utf8_lossy(message.payload());
            let libc::timespec{ tv_sec, tv_nsec } = message.timestamp();
            info!("message received from topic: {TOPIC:?} on source: {SOURCE:?}: Monotonic time - {tv_sec}.{tv_nsec:0>9}. Data - {payload}");
        }),
    )
    .unwrap();
    let _subscriber = Subscriber::try_new(
        &connection,
        config,
        Box::new(|e| match e {
            None => info!("Subscribed"),
            Some(e) => {
                error!("Not subscribed because {e:?}");
                abort();
            }
        }),
    )
    .unwrap();

    loop {
        sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use log::{debug, warn};

    use super::*;

    #[test]
    fn receives_analytics_scene_description_promptly() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let mut droppable_tx = Some(tx);

        let connection =
            Connection::try_new(Some(Box::new(|e| println!("Not connected because {e:?}"))))
                .unwrap();
        let config = SubscriberConfig::try_new(
            TOPIC,
            SOURCE,
            Box::new(move |message| {
                let payload = String::from_utf8(message.payload().to_vec());
                let Some(tx) = &droppable_tx else {
                    debug!("Dropping message because sender was previously dropped");
                    return;
                };
                if tx.try_send(payload).is_err() {
                    warn!("Dropping sender because receiver has been deallocated");
                    droppable_tx = None;
                }
            }),
        )
        .unwrap();
        let _subscriber = Subscriber::try_new(
            &connection,
            config,
            Box::new(|e| match e {
                None => println!("Subscribed"),
                Some(e) => println!("Not subscribed because {e:?}"),
            }),
        )
        .unwrap();

        let payload = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
        assert!(!payload.is_empty());
        println!("{payload}");
    }
}

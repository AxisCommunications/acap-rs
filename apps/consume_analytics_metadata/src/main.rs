use std::{ffi::CStr, process::abort, thread::sleep, time::Duration};

use log::{debug, error, info};
use mdb::{Connection, Subscriber, SubscriberConfig};

const TOPIC: &CStr = c"com.axis.analytics_scene_description.v0.beta";
const SOURCE: &CStr = c"1";

fn main() {
    app_logging::init_logger();

    let connection = Connection::try_new(Box::new(|e| {
        error!("Not connected because {e:?}");
        abort();
    }))
    .unwrap();

    let config = SubscriberConfig::try_new(
        TOPIC,
        SOURCE,
        Box::new(|message| {
            let payload = String::from_utf8_lossy(message.payload());
            debug!("Decoded payload {:?}", &payload);
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
    use log::warn;

    use super::*;

    #[test]
    fn receives_analytics_scene_description_promptly() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let mut droppable_tx = Some(tx);

        let connection =
            Connection::try_new(Box::new(|e| println!("Not connected because {e:?}"))).unwrap();
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

        let payload = rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        assert!(!payload.is_empty());
        println!("{payload}");
    }
}

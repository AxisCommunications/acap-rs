#![forbid(unsafe_code)]
//! A simple app that uses a VAPIX service account to access VAPIX APIs.

use std::time::Duration;

use acap_vapix::systemready;
use log::{debug, info};
use tokio::time;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    acap_logging::init_logger();
    let client = acap_vapix::local_client().unwrap();
    loop {
        debug!("Checking if system is ready");
        let data = systemready::systemready().execute(&client).await.unwrap();
        if data.system_ready() {
            if let Some(uptime) = data.uptime() {
                info!("System is ready after being up for {uptime:?}");
            } else {
                info!("System is ready");
            }
            break;
        } else {
            debug!("System is not ready, checking soon.");
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
#[cfg(test)]
mod tests {
    use acap_vapix::{
        applications_control, basic_device_info, parameter_management, systemready, ws_data_stream,
        ws_data_stream::{ContentFilter, TopicFilter},
    };

    #[tokio::test]
    async fn smoke_test_applications_control() {
        let client = acap_vapix::local_client().unwrap();
        let e = applications_control::control(applications_control::Action::Start, "foo")
            .execute(&client)
            .await
            .unwrap_err();
        let e = e.downcast::<applications_control::Error>().unwrap();
        let applications_control::Error::NotFound = e else {
            panic!("{e:?}")
        };
    }

    #[tokio::test]
    async fn smoke_test_basic_device_info() {
        let mut client = acap_vapix::local_client().unwrap();

        let properties = basic_device_info::Client::new(&client)
            .get_all_properties()
            .send()
            .await
            .unwrap();
        assert_eq!(properties.property_list.unrestricted.brand, "AXIS");

        let properties = basic_device_info::Client::new(&client)
            .get_properties(&["Brand"])
            .send()
            .await
            .unwrap();
        assert_eq!(properties.property_list.get("Brand").unwrap(), "AXIS");

        client = client.anonymous_auth();

        let properties = basic_device_info::Client::new(&client)
            .get_all_unrestricted_properties()
            .send()
            .await
            .unwrap();
        assert_eq!(properties.property_list.brand, "AXIS");
    }

    #[tokio::test]
    async fn smoke_test_parameter_management() {
        let client = acap_vapix::local_client().unwrap();
        // It is not guaranteed that this parameter will always exist with this value, but it seems
        // like it should be stable enough to be useful as a test.
        let params = parameter_management::list()
            .group("root.Brand.Brand")
            .execute(&client)
            .await
            .unwrap();
        assert_eq!(params.get("root.Brand.Brand").unwrap(), "AXIS")
    }

    #[tokio::test]
    async fn smoke_test_systemready() {
        let client = acap_vapix::local_client().unwrap();
        let data = systemready::systemready().execute(&client).await.unwrap();
        // TODO: Remove once parsed eagerly
        let _ = data.preview_mode();
        let _ = data.uptime();
    }

    #[tokio::test]
    async fn smoke_test_ws_data_stream() {
        let client = acap_vapix::local_client().unwrap();
        // It is not guaranteed that this event will always exist, but it seems like likely enough
        // to be useful as a test.
        let mut stream = ws_data_stream::events_configure()
            .event_filter((
                ContentFilter::unvalidated(
                    r##"boolean(//SimpleItem[@Name="port" and @Value="1"])"##,
                ),
                TopicFilter::unvalidated("tns1:Device/tnsaxis:IO/VirtualInput"),
            ))
            .execute(&client)
            .await
            .unwrap();
        let mut notification = stream.try_next().await.unwrap();
        let port = notification.message.source.remove("port").unwrap();
        assert_eq!(port, "1");
    }
}

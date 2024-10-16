_Demo of running a MQTT broker and client_

> [!IMPORTANT]
> `mosquitto` is not part of the ACAP SDK and may be removed at any time.

A bonus feature is inspecting the messages sent from the MQTT Event Bridge.
To do so, configure the MQTT Client like:

<!-- TODO: Don't use hard coded secrets -->
- Host: `<AXIS_DEVICE_IP>` (neither `localhost` nor `127.0.0.1` are accepted).
- Protocol: `MQTT over SSL`
- Port: `8884`
- Username: `d7a3407e-f275-4137-8530-1868d9aab6b5`
- Password: `683ee671c6db83a31c0600f10eaf28d067de9600`

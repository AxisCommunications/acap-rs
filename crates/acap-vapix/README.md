_VAPIX access for ACAP apps_

Please see the [VAPIX](https://www.axis.com/vapix-library/)
and [ACAP](https://axiscommunications.github.io/acap-documentation/docs/develop/VAPIX-access-for-ACAP-applications.html)
documentation for more information about the APIs and how to access them from an ACAP app, respectively.

## Example

```no_run
use acap_vapix::systemready;

#[::tokio::main]
async fn main() {
    let client = acap_vapix::local_client().unwrap();
    if let Some(uptime) = systemready::systemready()
        .timeout(10)
        .execute(&client)
        .await
        .unwrap()
        .uptime()
    {
        println!("System has been up for {uptime:?}");
    }
    // ... make more VAPIX calls with the client.
}
```

## Status

Bindings are typically implemented as they are needed.
This table is an attempt at providing an overview of what exists and how usable it is.

| Name        | Methods | Status       |
|-------------|---------|--------------|
| Systemready | 1/2     | Experimental |

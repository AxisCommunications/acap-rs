# ACAP for Rust

_Easy and safe [ACAP] apps using [Rust]_

> [!IMPORTANT]
> This project is an experiment provided "as is".
> While we strive to maintain it, there's no guarantee of ongoing support, and it may become unmaintained in the future.
> Your contributions are appreciated, and feel free to fork and continue the journey if needed.

This repo is home to a mixture of developer tools, example apps, and library crates.
To simply get started with a new app, please see [acap-rs-app-template](https://github.com/AxisCommunications/acap-rs-app-template).

## Quickstart guide

The quickest way to build the `hello_world` example is to launch the dev container and run `make build AXIS_PACKAGE=hello_world`.
Once it completes there should be two `.eap` files in `target/acap`:

```console
$ ls -1 target/acap
hello_world_1_0_0_aarch64.eap
```

If you prefer to not use dev containers, or the implementation in your favorite IDE is buggy, the app can be built using only `docker`:

```sh
docker build --file .devcontainer/Dockerfile --tag acap-rs .
docker run \
  --interactive \
  --rm \
  --tty \
  --user $(id -u):$(id -g) \
  --volume $(pwd):$(pwd) \
  --workdir $(pwd) \
  acap-rs \
  make build AXIS_PACKAGE=hello_world
```

This works with any of the [example applications](#example-applications).

Important workflows are documented in the [Makefile](./Makefile) and can now be listed with `make help`.

## Advanced setup
Development environments outside containers are more difficult to reproduce and maintain.
Should it nonetheless be of interest, one procedure is documented in [this workflow](.github/workflows/on-host-workflow.yml).

## Example applications

Below is the list of examples available in the repository.

* [`consume_analytics_metadata`](apps/consume_analytics_metadata/src/main.rs)
: An example that consumes metadata.
* [`embedded_web_page`](apps/embedded_web_page/src/main.rs)
: An example that illustrates how to bundle an embedded web page.
* [`hello_world`](apps/hello_world/src/main.rs)
: A simple "Hello, World!" application.
* [`licensekey_handler`](apps/licensekey_handler/src/main.rs)
: An example that illustrates how to check the licensekey status.
* [`using_a_build_script`](apps/using_a_build_script/src/main.rs)
: Uses a build script to generate html, lib and app manifest files at build time.
* [`vapix_access`](apps/vapix_access/src/main.rs)
: Uses a VAPIX service account to access VAPIX APIs.

## Library crates

| Name           | Documentation                                                   |
|----------------|-----------------------------------------------------------------|
| acap-logging   | [on docs.rs](https://docs.rs/acap-logging/latest/acap_logging/) |
| acap-vapix     | [in source](crates/acap-vapix/src/lib.rs)                       |
| licensekey     | [in source](crates/licensekey/src/lib.rs)                       |
| licensekey-sys |                                                                 |
| mdb            |                                                                 |
| mdb-sys        |                                                                 |

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

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

## Testing
Some items in this workspace rely on libraries or hardware on Axis cameras. This makes testing difficult since these tests cannot run on an arbitrary x86_64 host. Below are some steps to enable running unit test on device.

1. Connect an Axis camera to your network and ensure it is accessible.
2. The user will likely need to be `root`, such that the Axis camera file system is writable.
3. Set the `CARGO_TEST_CAMERA` environment variable to the user and IP of the camera with the format `user@ip`
4. Set up an identity based SSH connection to the camera.
  1. Create an ID via `ssh-keygen`
  2. Copy the id to the device via `ssh-copy-id`

Now, via the [remote-test.sh](remote-test.sh) script, and the `runner = ["/workspaces/acap-rs/remote-test.sh"]` line in the .cargo/config.toml, tests with the `aarch64-unknown-linux-gnu` target triplet will automatically be copied to the remote camera and executed there. Run these tests via `cargo test --target aarch64-unknown-linux-gnu`.

If you want to run tests locally, just make sure you clear the `CARGO_TEST_CAMERA` environment variable via `unset CARGO_TEST_CAMERA`.

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
* [`reverse_proxy`](apps/reverse_proxy/src/main.rs)
: Uses a web server and reverse proxy configuration to expose HTTP and WebSocket APIs.
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

## Troubleshooting

The docker image may fail to build with the following included in the output:
`/usr/bin/env: 'sh\r': No such file or directory`
This is likely caused by `git` replacing POSIX newlines with Windows newlines in which case it can be resolved by either

- cloning the code in Windows Subsystem for Linux (WSL), or
- reconfiguring `git`.

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

# ACAP for Rust

_Easy and safe [ACAP] apps using [Rust]_

> [!IMPORTANT]
> This project is an experiment provided "as is".
> While we strive to maintain it, there's no guarantee of ongoing support, and it may become
> unmaintained in the future.
> Your contributions are appreciated, and feel free to fork and continue the journey if needed.

This repository provides the developer tools, libraries, and documentation that form ACAP for Rust platform.
To quickly start a new app, see [acap-rs-app-template](https://github.com/AxisCommunications/acap-rs-app-template).

## Table of Contents

- [**Getting started**](#getting-started)
  - [Dev container](#dev-container)
  - [Non-dev container](#non-dev-container)
  - [Without container](#without-container)
- [**Developer tools**](#developer-tools)
  - [Porcelain programs](#porcelain-programs)
  - [Plumbing programs](#plumbing-programs)
- [**Libraries**](#libraries)
  - [ACAP Native API bindings](#acap-native-api-bindings)
  - [VAPIX API bindings](#vapix-api-bindings)
  - [Other library crates](#other-library-crates)
- [**Documentation**](#documentation)
  - [Example applications](#example-applications)
- [**Troubleshooting**](#troubleshooting)

## Getting started

The recommended setup is using the [dev container](#dev-container) and the most recent LTS version of AXIS OS [^1].

### Dev container

The quickest way to build the `hello_world` example is to launch the dev container and
run `make build AXIS_PACKAGE=hello_world`.
Once it completes there should be an `.eap` file in `target/acap`:

```console
$ ls -1 target/acap
hello_world_1_0_0_aarch64.eap
```

This works with any of the [example applications](#example-applications).

Important workflows are documented in the [Makefile](./Makefile) and can now be listed with
`make help`.

### Non-dev container

If you prefer to not use dev containers, or the implementation in your favorite IDE is buggy, the
app can be built using
only `docker`:

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

### Without container

Development environments outside containers are more difficult to reproduce and maintain.
Should it nonetheless be of interest, one procedure is documented
in [this workflow](.github/workflows/on-host-workflow.yml).

## Developer tools

Tools for developing ACAP apps are provided primarily as binary crates.
The tools can be roughly divided into low level _plumbing_ and high level _porcelain_.

All of these tools are installed in the dev container of this project.
To install them in another project before they are released, use the `--git` option e.g. like:

```sh
cargo install --locked --git https://github.com/AxisCommunications/acap-rs.git cargo-acap-sdk
```

### Porcelain programs

The focus of these tools are to make common things easy.

- `cargo-acap-sdk` - Automation of common development workflows.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/cargo-acap-sdk/README.md)

### Plumbing programs

The focus of these tools are to make less common things possible.

- `acap-ssh-utils` - Utilities for interacting with Axis devices over SSH.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/acap-ssh-utils/README.md)
- `cargo-acap-build`: Build utilities for ACAP apps and other executables deployed to Axis devices.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/cargo-acap-build/README.md)
- `device-manager`: Utilities for manipulating a single Axis device.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/device-manager/README.md)
- `fleet-manager`: Utilities for manipulating multiple Axis devices.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/fleet-manager/README.md)

These can be installed independently and are provided as library crates too for developers who want
to write their own,
tailored tools.

## Libraries

To make it easier to relate the Rust offering to the official offering, the library crates are
grouped in a similar way as in the ACAP Native SDK APIs documentation.

> [!NOTE]
> If an API that is important to you is missing, create or upvote the feature request for it.

### ACAP Native API bindings

These are idiomatic and safe bindings for the [C APIs](https://axiscommunications.github.io/acap-documentation/docs/api/native-sdk-api.html#native-sdk-api).
Each crate has a corresponding `*-sys`, which is omitted for brevity.

- `axparameter`: Bindings for the Parameter API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/axparameter/src/lib.rs)
- `axevent`: Bindings for the Event API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/axevent/src/lib.rs)
- `axoverlay`: Bindings for the Overlay API
  - Status: 📄 Not started
- `axserialport`: Bindings for the Serial Port API
  - Status: 📄 Not started
- `axstorage`: Bindings for the Edge Storage API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/axstorage/src/lib.rs)
- `bbox`: Bindings for the Bounding Box API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/bbox/src/lib.rs)
- `larod`: Bindings for the Machine Learning API.
  - Status: 💡 Started
  - Documentation: [Pull request](https://github.com/AxisCommunications/acap-rs/pull/120)
- `licensekey`: Bindings for the License Key API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/licensekey/src/lib.rs)
- `mdb`: Bindings for the Message Broker API.
  - Status: ⚠️ Alpha
  - Documentation: [Source code](crates/mdb/src/lib.rs)
- `vdo`: Bindings for the Video Capture API.
  - Status: 💡 Started
  - Documentation: [Pull request](https://github.com/AxisCommunications/acap-rs/pull/153)

### VAPIX API bindings

All the VAPIX APIs are provided by a single crate:

- `acap-vapix`: Bindings for various VAPIX APIs + credentials lookup.
  - Status: ⚠️ Alpha
  - Documentation: [README](crates/acap-vapix/README.md)

### Other library crates

These are not closely related to any official APIs but may nonetheless be helpful in their own way:

- `acap-logging`: Logging utilities for ACAP applications
  - Status: 🚧 Beta
  - Documentation: [Docs.rs](https://docs.rs/acap-logging/latest/acap_logging/)

## Documentation

Ideally information is provided when and where the reader needs it, such as:

- Tab completions and help texts for binaries.
- Docstrings and doctests for libraries.

This is however not always suitable, and this section lists other sources of documentation provided
by this project.

### Example applications

- `axparameter_example`: Creates, reads, updates, deletes, and subscribes to parameters.
  - Status: ⚠️ Alpha
  - [Source code](apps/axparameter_example/src/main.rs)
- `axstorage_example`: Writes data to files on all connected storages.
  - Status: ⚠️ Alpha
  - [Source code](apps/axstorage_example/src/main.rs)
- `bounding_box_example`: Draws simple overlays in video streams.
  - Status: ⚠️ Alpha
  - [Source code](apps/bounding_box_example/src/main.rs)
- `consume_analytics_metadata`: Subscribes to _analytics scene description_ data using `mdb`.
  - Status: ⚠️ Alpha
  - [Source code](apps/consume_analytics_metadata/src/main.rs)
- `embedded_web_page`: Bundles an embedded web page.
  - Status: ⚠️ Alpha
  - [Source code](apps/embedded_web_page/src/main.rs)
- `hello_world`:Sets up and uses logging using common functions and `acap-logging`.
  - Status: ⚠️ Alpha
  - [Source code](apps/hello_world/src/main.rs)
- `licensekey_handler`:Checks if an app is licensed using `licensekey`.
  - Status: ⚠️ Alpha
  - [Source code](apps/licensekey_handler/src/main.rs)
- `reverse_proxy`: Exposes HTTP and WebSocket APIs using a `axum` and reverse proxy configuration.
  - Status: ⚠️ Alpha
  - [Source code](apps/reverse_proxy/src/main.rs)
- `send_event`: Sends events using `axevent`.
  - Status: ⚠️ Alpha
  - [Source code](apps/send_event/src/main.rs)
- `using_a_build_script`: Generates html, lib and app manifest files using a build script.
  - Status: ⚠️ Alpha
  - [Source code](apps/using_a_build_script/src/main.rs)
- `vapix_access`: Accesses VAPIX APIs using `acap-vapix`.
  - Status: ⚠️ Alpha
  - [Source code](apps/vapix_access/src/main.rs)

<!-- inspect_env is omitted because it is intended primarily as a test -->

### Articles

- [Running apps and tests on device](docs/running-apps-and-tests-on-device.md)

## Troubleshooting

The docker image may fail to build with the following included in the output:
`/usr/bin/env: 'sh\r': No such file or directory`
This is likely caused by `git` replacing POSIX newlines with Windows newlines in which case it can
be resolved by either

- cloning the code in Windows Subsystem for Linux (WSL), or
- reconfiguring `git`.

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/

[Rust]: https://doc.rust-lang.org/

[^1]: Compatibility with various host and target environments are explored and explained in [compatibility](docs/compatibility.md)

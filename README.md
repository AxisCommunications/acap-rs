# ACAP for Rust

_Easy and safe [ACAP] apps using [Rust]_

> [!IMPORTANT]
> This project is an experiment provided "as is".
> While we strive to maintain it, there's no guarantee of ongoing support, and it may become
> unmaintained in the future.
> Your contributions are appreciated, and feel free to fork and continue the journey if needed.

This repo is home to a mixture of developer tools, libraries, and documentation.
To simply get started with a new app, please
see [acap-rs-app-template](https://github.com/AxisCommunications/acap-rs-app-template).

## Table of Contents

- [**Getting started**](#getting-started)
    - [Building without Dev Containers](#building-without-dev-containers)
    - [Advanced setup](#advanced-setup)
- [**Tools**](#tools)
- [**Libraries**](#libraries)
    - [ACAP Native APIs](#acap-native-apis)
    - [Novel APIs](#novel-apis)
- [**Documentation**](#documentation)
    - [Example applications](#example-applications)

## Getting started

The quickest way to build the `hello_world` example is to launch the dev container and
run `make build AXIS_PACKAGE=hello_world`.
Once it completes there should be two `.eap` files in `target/acap`:

```console
$ ls -1 target/acap
hello_world_1_0_0_aarch64.eap
```

This works with any of the [example applications](#example-applications).

Important workflows are documented in the [Makefile](./Makefile) and can now be listed with
`make help`.

### Containerize build without without Dev Containers

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

### Advanced setup

Development environments outside containers are more difficult to reproduce and maintain.
Should it nonetheless be of interest, one procedure is documented
in [this workflow](.github/workflows/on-host-workflow.yml).

## Binary crates

Tools for developing ACAP apps are provided primarily as the following binary crates:.
The tools can be roughly divided into low level _plumbing_ and high level _porcelain_.

### Porcelain programs

- `cargo-acap-sdk` - Tools for developing ACAP apps using Rust.
    - Status: ⚠️ Experimental
    - Documentation: [README](crates/cargo-acap-sdk/README.md)

Notes:

- Provided as a binary crate only.
- Makes common tasks easy.
- Operates on Cargo packages:
    - Acts on any number of packages using Cargo's package selection rules.
    - Infers app name from package.

![porcelain commands](docs/img/porcelain-commands.svg)
![porcelain commands](docs/img/porcelain-commands.dark.svg)
![porcelain commands](docs/img/porcelain-commands.light.svg)

### Plumbing programs

- `acap-ssh-utils` - Utilities for interacting with Axis devices over SSH.
    - Status: ⚖️ Stable
    - Documentation: [README](crates/acap-ssh-utils/README.md)
- `cargo-acap-build`: Build utilities for ACAP apps and other executables deployed to Axis devices.
    - Status: ⚖️ Stable
    - Documentation: [README](crates/cargo-acap-build/README.md)
- `device-manager`: Utilities for manipulating Axis devices.
    - Status: ⚖️ Stable
    - Documentation: [README](crates/device-manager/README.md)

These can be installed independently and are provided as library crates too for developers who want
to write their own,
tailored tools.

Notes:

- Provided primarily as library crates for use in porcelain binary crates.
- Binary crates are provided as well to facilitate less common tasks, independent of programming
  language.
- Since `eap-install.sh` already provides a CLI for controlling applications, it is provided as a
  library only.
- Operates on apps, identified by a path to an EAP or a name, instead of Cargo packages.

![plumbing commands](docs/img/plumbing-commands.svg)

## Library crates

To make it easier to relate the Rust offering to the official offering, the library crates are
grouped in a similar way as in the ACAP Native SDK APIs documentation.

> [!NOTE]
> If an API that is important to you is missing, create or upvote the feature request for it.

Notes:

- To ensure that the project can continue to operate as open source, libraries primarily target
  official APIs.

### ACAP Native API bindings

Idiomatic and safe bindings for official APIs.

- `axevent`: Bindings for the Event API.
    - Status: ⚖️ Stable
- `licensekey`: Bindings for the License Key API.
    - Status: ⚖️ Stable
- `mdb`: Bindings for the Message Broker API.
    - Status: ⚖️ Stable
        - License Key API:

Notes:

- These are the most valuable libraries because they are the most difficult to write.
- The difficulty comes from needing to use unsafe rust which is notoriously error-prone.

### VAPIX API bindings

- `acap-vapix`: Bindings for various VAPIX APIs + credentials lookup.
    - Documentation: [README](crates/acap-vapix/README.md)

Notes:

- All APIs are provided by the same crate, `acap-vapix`, and features are used to control
  dependencies.
- Features can also be used to choose between `reqwest` and cURL, which is less commonly used than
  `reqwest` but **may** help reduce binary size.

### Other library crates

These are not closely related to any official APIs but may nonetheless be helpful in their own way:

- `acap-logging`: Logging utilities for ACAP applications
    - Status: ⚖️ Stable
    - Documentation: [Docs.rs](https://docs.rs/acap-logging/latest/acap_logging/)
- `acap-dirs`: Makes documentation about paths available straight in the IDE.

## Documentation

Ideally information is provided when and where the reader needs it, such as:

- Tab completions and help texts for binaries.
- Docstrings and doctests for libraries.

This is however not always suitable, and this section lists other sources of documentation provided
by this project.

### Example applications

- `consume_analytics_metadata`: Subscribes to _analytics scene description_ data using `mdb`.
    - Status: ⚖️ Stable
    - [Source code](apps/consume_analytics_metadata/src/main.rs)
- `embedded_web_page`: Bundles an embedded web page.
    - Status: ⚖️ Stable
    - [Source code](apps/embedded_web_page/src/main.rs)
- `hello_world`:Sets up and uses logging using common functions and `acap-logging`.
    - Status: ⚖️ Stable
    - [Source code](apps/hello_world/src/main.rs)
- `licensekey_handler`:Checks if an app is licensed using `licensekey`.
    - Status: ⚖️ Stable
    - [Source code](apps/licensekey_handler/src/main.rs)
- `reverse_proxy`: Exposes HTTP and WebSocket APIs using a `axum` and reverse proxy configuration.
    - Status: ⚖️ Stable
    - [Source code](apps/reverse_proxy/src/main.rs)
- `send_event`: Sends events using `axevent`.
    - Status: ⚖️ Stable
    - [Source code](apps/send_event/src/main.rs)
- `using_a_build_script`: Generates html, lib and app manifest files using a build script.
    - Status: ⚖️ Stable
    - [Source code](apps/using_a_build_script/src/main.rs)
- `vapix_access`: Accesses VAPIX APIs using `acap-vapix`.
    - Status: ⚖️ Stable
    - [Source code](apps/vapix_access/src/main.rs)

### Articles

- _How to optimize the size of Rust programs_

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

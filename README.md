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

## Related projects

This is not the only initiative to facilitate building Rust applications for Axis devices.
Below is a survey of other projects known to the author.

### cargo-acap

[cargo-acap] is a Rust binary crate that cross-compiles and packages Rust programs for Axis devices.
Installing it and building eap files for virtually every architecture can be done like
```sh
cargo install cargo-acap
cargo acap build
```

Praises
* Depends only on Cargo et al and Docker.
* Requires no ACAP specific boilerplate.
* Supports old products, including ARTPEC-4 and ARTPEC-5.

Reasonable complaints
* Unaware of the ACAP manifest.
  * Does not validate the ACAP manifest.
  * Requires information to be duplicated in `Cargo.toml` if the ACAP manifest is to be used.
  * Does not use dynamic user.
* Requires Docker
  * Cannot easily be used from within a Docker container.
* The default docker image cannot be used to link the ACAP SDK APIs.
* Assumes custom target triples with the vendor set to `axis`.
  * Restricts how docker images can be built.
  * May cause bugs in crates that use conditional compilation[^1].
* Does not support workspace projects
* Does not support bundling target-specific files[^2].

Unreasonable complaints
* Does not facilitate any interaction with the device including
  * compiling and running tests
  * installing the built `.eap` file

[cargo-acap]: https://github.com/trunnion/cargo-acap
[^1]: https://github.com/trunnion/cargo-acap/commit/6748c52ef1c13a6a12cc327a65a333c012c5725b
[^2]: This has been discussed but not implemented https://github.com/trunnion/cargo-acap/pull/5

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

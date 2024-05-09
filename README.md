# ACAP for Rust

_Easy and safe [ACAP] apps using [Rust]_

> [!IMPORTANT]
> This project is an experiment provided "as is".
> While we strive to maintain it, there's no guarantee of ongoing support, and it may become unmaintained in the future.
> Your contributions are appreciated, and feel free to fork and continue the journey if needed.

## Quickstart guide

Ensure global prerequisites are installed:

* Docker
* Rust e.g. [using rustup](https://www.rust-lang.org/tools/install)
* Python e.g. using [pyenv](https://github.com/pyenv/pyenv)

Create, activate and populate the local development environment like

```sh
source ./init_env.sh
make sync_env
```

Build the `hello_world` example and create `.eap` files in the `target/acap/` directory like

```sh
PACKAGE=hello_world make build
```

This works with any of the [example applications](#example-applications).

Other important workflows are documented in the [Makefile](./Makefile) and can be listed with `make help`.

## Example applications

Below is the list of examples available in the repository.

* [`hello_world`](apps/hello_world/src/main.rs)
: A simple "Hello, World!" application.
* [`licensekey_handler`](apps/licensekey_handler/src/main.rs)
: An example that illustrates how to check the licensekey status.

## Related projects

This is not the only initiative to facilitate building Rust applications for Axis devices.
Below is a survey of other projects known to the author.

### [cargo-acap](https://github.com/trunnion/cargo-acap)

A Rust binary crate that facilitates cross-compiling and bundling Rust programs for Axis devices.
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
* Requires Docker
  * Cannot easily be used from within a Docker container.
* The default docker image cannot be used to link the ACAP SDK APIs.
* Assumes custom target triples with the vendor set to `axis`.
  * Restricts how docker images can be built.
  * May cause bugs in crates that use conditional compilation[^1].
* Does not support workspace projects

Unreasonable complaints
* Does not facilitate any interaction with the device including
  * compiling and running tests
  * installing the built `.eap` file

[^1]: https://github.com/trunnion/cargo-acap/commit/6748c52ef1c13a6a12cc327a65a333c012c5725b

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

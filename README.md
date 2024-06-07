# ACAP for Rust

_Easy and safe [ACAP] apps using [Rust]_

> [!IMPORTANT]
> This project is an experiment provided "as is".
> While we strive to maintain it, there's no guarantee of ongoing support, and it may become unmaintained in the future.
> Your contributions are appreciated, and feel free to fork and continue the journey if needed.

## Quickstart guide

Build the `hello_world` example and create `.eap` files in the `target/acap/` directory like

```sh
docker build --tag acap-rs .
docker run \
  --interactive \
  --rm \
  --tty \
  --user $(id -u):$(id -g) \
  --volume $(pwd):$(pwd) \
  --workdir $(pwd) \
  acap-rs \
  make build PACKAGE=hello_world
```

This works with any of the [example applications](#example-applications).

## Advanced setup

Ensure global prerequisites are installed:

* Docker
* Rust e.g. [using rustup](https://www.rust-lang.org/tools/install)
* Python e.g. using [pyenv](https://github.com/pyenv/pyenv)

Create, activate and populate the local development environment like

```sh
source ./init_env.sh
make sync_env
```

Important workflows are documented in the [Makefile](./Makefile) and can now be listed with `make help`.

## Example applications

Below is the list of examples available in the repository.

* [`hello_world`](apps/hello_world/src/main.rs)
: A simple "Hello, World!" application.
* [`licensekey_handler`](apps/licensekey_handler/src/main.rs)
: An example that illustrates how to check the licensekey status.

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

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

Build the hello_world example and create `.eap` files in the `target/acap/` directory like

```sh
PACKAGE=hello_world make build
```

Other important workflows are documented in the [Makefile](./Makefile) and can be listed with `make help`.

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

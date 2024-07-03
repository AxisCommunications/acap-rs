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
docker build --tag acap-rs .
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

## Advanced setup

Ensure global prerequisites are installed:

* Docker
* Rust e.g. [using rustup](https://www.rust-lang.org/tools/install)
* Python e.g. using [pyenv](https://github.com/pyenv/pyenv)
  * When using the system python on some versions of Debian and Ubuntu, it is necessary to `apt install python3-venv`.
* `scp`, `ssh`, and `sshpass` (needed only for `make run`)

Create, activate and populate the local development environment like

```sh
source ./init_env.sh
make sync_env
```

Important workflows are documented in the [Makefile](./Makefile) and can now be listed with `make help`.

## Example applications

Below is the list of examples available in the repository.

* [`embedded_web_page`](apps/embedded_web_page/src/main.rs)
: An example that illustrates how to bundle an embedded web page.
* [`hello_world`](apps/hello_world/src/main.rs)
: A simple "Hello, World!" application.
* [`licensekey_handler`](apps/licensekey_handler/src/main.rs)
: An example that illustrates how to check the licensekey status.

## Binary crates

```console
$ acap-ssh-utils help
Utilities for interacting with Axis devices over SSH.

The commands assume that the user has already
- installed `scp`, `ssh` and `sshpass`,
- added the device to the `known_hosts` file,
- enabled SSH on the device,
- configured the SSH user with a password and the necessary permissions, and
- installed any apps that will be impersonated.

Usage: acap-ssh-utils --host <HOST> --user <USER> --pass <PASS> <COMMAND>

Commands:
  patch      Patch app on device
  run-app    Run app on device, sending output to the terminal
  run-other  Run any executable on device, sending output to the terminal
  help       Print this message or the help of the given subcommand(s)

Options:
      --host <HOST>
          Hostname or IP address of the device

          [env: AXIS_DEVICE_IP=]

  -u, --user <USER>
          The username to use for the ssh connection

          [env: AXIS_DEVICE_USER=]

  -p, --pass <PASS>
          The password to use for the ssh connection

          [env: AXIS_DEVICE_PASS=]

  -h, --help
          Print help (see a summary with '-h')
```

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

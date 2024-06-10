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
* `scp`, `ssh`, and `sshpass` (not needed for `build`ing)

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

## Application structure

- `{project_root}` Vaguely defined as where the parent of `.git` and the workspace `Cargo.toml`.
- `{package_dir}` Can be named anything, but it is helpful if it evokes the name of the app.
  For single-app projects this typically coincides with `{project_root}`.
  - `Cargo.toml`
  - `LICENSE` License to include in the `.eap` file. Required by `acap-build`.
  - `build.rs` The build script can be used to dynamically prepare files to be included.
     The name can be anything but must match `Cargo.toml::packe.build` and `build.rs` is the conventional name.
  - `manifest.json` Manifest to include in the `.eap` file. May be read by the cargo plugin in the future.
  - `additional-files` Files to be included in the `.eap` file. Typically static and tracked in SCM.
  - `src`
    - `main.rs` I think this can be named anything if the correct settings are made in `Cargo.toml` but `main.rs` is the conventional name.
- `{target_dir}` Typically called `target` and located in `{project_root}`.
  - `**` Zero or more descendants
    - `{out_dir}` Created by Cargo during compilation and the path is known only to the build script.
      - `additional-files` Files to be included in the `.eap`.

Note that tools may assume that
- `Cargo.toml::package.name` matches `manifest.json::acapPackageConf.setup.appName`.
- `Cargo.toml::package.version` matches `manifest.json::acapPackageConf.setup.version`.

## License

[MIT](LICENSE)

[ACAP]: https://axiscommunications.github.io/acap-documentation/
[Rust]: https://doc.rust-lang.org/

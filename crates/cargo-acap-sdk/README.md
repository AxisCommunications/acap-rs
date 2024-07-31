```console
$ cargo-acap-sdk  -h
Tools for developing ACAP apps using Rust.

Usage: cargo-acap-sdk <COMMAND>

Commands:
  build         Build app(s) for release
  run           Build app(s) and run on the device
  test          Build test(s) and run on the device
  install       Build app(s) for release and install on the device
  start         Start app(s) on the device
  stop          Stop app(s) on the device
  uninstall     Uninstall app(s) on the device
  completions   Print shell completion script for this program
  reinit        Restore and initialize device to a known, useful state
  help          Print this message or the help of the given subcommand(s)

Options:
      --file <FILE>    Location of `Dockerfile` to build in.
      --image <IMAGE>  Name of container image to build in.
      --host <HOST>    Hostname or IP address of the device [env: AXIS_DEVICE_IP=]
  -u, --user <USER>    The username to use for the connections [env: AXIS_DEVICE_USER=]
  -p, --pass <PASS>    The password to use for the connections [env: AXIS_DEVICE_PASS=]
  -h, --help           Print help
```

```console
$ cargo-acap-sdk -h
ACAP analog to `cargo` for building and deploying apps

Usage: cargo-acap-sdk <COMMAND>

Commands:
build        Build app(s)
run          Build executable for app(s) and run on the device, impersonating a user or the app
test         Build test(s) and run on the device, impersonating a user or the app
install      Build app(s) and install on the device
completions  Print shell completion script for this program
help         Print this message or the help of the given subcommand(s)

Options:
      --host <HOST>  Hostname or IP address of the device [env: AXIS_DEVICE_IP=]
  -u, --user <USER>  The username to use for the ssh connection [env: AXIS_DEVICE_USER=]
  -p, --pass <PASS>  The password to use for the ssh connection [env: AXIS_DEVICE_PASS=]
  -h, --help         Print help
```

```console
$ device-manager -h 
Utilities for managing individual devices.

Usage: device-manager --host <HOST> --user <USER> --pass <PASS> <COMMAND>

Commands:
  restore  Restore device to a clean state
  reinit   Restore and initialize device to a known, useful state
  help     Print this message or the help of the given subcommand(s)

Options:
      --host <HOST>  Hostname or IP address of the device [env: AXIS_DEVICE_IP=]
  -u, --user <USER>  The username to use for the ssh connection [env: AXIS_DEVICE_USER=]
  -p, --pass <PASS>  The password to use for the ssh connection [env: AXIS_DEVICE_PASS=]
  -h, --help         Print help
```

Currently, this crate focuses on restoring devices to a known, useful state.
It decomposes the problem into two parts:

- _restore_ any device in any state to a minimal baseline configuration.
- _initialize_ a restored device to a more useful baseline configuration.

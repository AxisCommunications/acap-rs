```console
$ acap-ssh-utils -h      
Utilities for interacting with Axis devices over SSH.

Usage: acap-ssh-utils [OPTIONS] --host <HOST> <COMMAND>

Commands:
  patch      Patch app on device
  run-app    Run app on device, sending output to the terminal
  run-other  Run any executable on device, sending output to the terminal
  help       Print this message or the help of the given subcommand(s)

Options:
      --host <HOST>  Hostname or IP address of the device [env: AXIS_DEVICE_IP=]
  -u, --user <USER>  The username to use for the ssh connection [env: AXIS_DEVICE_USER=] [default: root]
  -p, --pass <PASS>  The password to use for the ssh connection [env: AXIS_DEVICE_PASS=] [default: pass]
  -h, --help         Print help (see more with '--help')
  -V, --version      Print version
```

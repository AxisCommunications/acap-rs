```console
$ fleet-manager -h
Utilities for managing devices in bulk

Usage: fleet-manager [FLEET] <COMMAND>

Commands:
  adopt     Add a device to the chosen fleet
  abandon   Restore selected device(s) and remove from the chosen fleet
  for-each  Run the provided command on selected devices, in parallel or in sequence
  reinit    Restore and initialize selected device(s) to a known, useful state
  help      Print this message or the help of the given subcommand(s)

Arguments:
  [FLEET]  Location of database file

Options:
  -h, --help     Print help
  -V, --version  Print version
```

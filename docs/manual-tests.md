This is not an exhaustive list, but the use cases listed here should work.


## Run Hello World

Note that this only works on AxisOS <12 as of writing.
As a workaround the `run` command can be replaced by SSHing into the device and running the binary manually:

```console
$ RUST_LOG=debug /usr/local/packages/hello_world/hello_world
```

```console
$ cargo-acap-sdk install -- -p hello_world
$ cargo-acap-sdk run -- -p hello_world
```

All records except "Hello trace!" should be emitted and appropriately colored.

## Start Hello World without default features

On the device:
```console
$ journalctl -f
```

```console
$ cargo-acap-sdk install -- -p hello_world
$ cargo-acap-sdk run -- -p hello_world
```

All records except "Hello trace!" should be emitted and appropriately colored in the output on the device.

The size of the executable should be noticably smaller than the one with default features.

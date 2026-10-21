Some items in this workspace rely on libraries or hardware on Axis cameras.
This makes testing difficult since these tests cannot run on an arbitrary x86_64 host.
To facilitate testing on Axis camera hardware, there are two approaches to testing.
Regardless of which approach is used, an Axis camera must be accessible on the network and be configured with a known password.

## Using `cargo-acap-sdk`
The `cargo-acap-sdk` tool ultimately uses `cargo-acap-build` to build a crate, then uses `acap-ssh-utils` to copy the resulting binary to the camera and execute it. The test binary is executed with `RUST_LOG=debug` but the `--nocapture` argument is not passed to the binary, so `println!` debugging statements will not be printed to stdout.

To use this test method, change the working directory to the top level directory of the crate to test. Then simply call run the command
`cargo-acap-sdk test --host <host-IP> --user <user> --pass <pass>`.

## Using cargo test runner
Cargo can be configured to invoke a specified runner whenever it would normally execute compiled code such as `cargo run` or `cargo test`, via [`target.<triple>.runner`](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner). A runner is an executable (or script) that cargo calls with the path to the compiled binary as its first argument, followed by any arguments passed to that binary.

The basic steps to set up the runner are:
1. Connect an Axis camera to your network and ensure it is accessible.
2. Set the `AXIS_DEVICE_IP` and, optionally, `AXIS_DEVICE_USER` environment variables.
3. Set up an identity based SSH connection to the camera.
   1. Create an ID via `ssh-keygen`
   2. Copy the id to the device via `ssh-copy-id`
4. Create a runner script, for example `remote-test.sh`, somewhere on your `PATH`:

```sh
#!/bin/sh
set -eu

if [ -n "${AXIS_DEVICE_IP:-}" ]; then
    LOCAL_PATH="$1"
    shift

    REMOTE_PATH=/tmp/$(basename "$LOCAL_PATH")
    CARGO_TEST_CAMERA="${AXIS_DEVICE_USER:-root}@${AXIS_DEVICE_IP}"
    scp -p "$LOCAL_PATH" "$CARGO_TEST_CAMERA:$REMOTE_PATH"
    ssh "$CARGO_TEST_CAMERA" ${REMOTE_ENV:-} "$REMOTE_PATH" "$@"
else
    exec "$@"
fi
```

5. Add the runner to `.cargo/config.toml` (create the file if needed):

```toml
[target.'cfg(all(not(miri), not(target_arch = "x86_64"), not(target_os = "macos")))']
runner = ["remote-test.sh"]
```

Tests targeting Axis device architectures are then copied to the remote camera and executed there automatically. Run them via `cargo test --target aarch64-unknown-linux-gnu`. This approach is particularly useful when only a specific crate is to be tested, or when `println!` debugging is used. An example invocation: `cargo test -p acap-vapix --tests -- --nocapture`.

Note that some programs, such as `consume_analytics_metadata`, do not work when running as root.

If you want to run tests locally, clear the `AXIS_DEVICE_IP` environment variable via `unset AXIS_DEVICE_IP`.

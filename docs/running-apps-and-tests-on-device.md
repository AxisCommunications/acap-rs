Some items in this workspace rely on libraries or hardware on Axis cameras.
This makes testing difficult since these tests cannot run on an arbitrary x86_64 host.
To facilitate testing on Axis camera hardware, there are two approaches to testing.
Regardless of which approach is used, an Axis camera must be accessible on the network and be configured with a known password.

## Using `cargo-acap-sdk`
The `cargo-acap-sdk` tool ultimately uses `cargo-acap-build` to build a crate, then uses `acap-ssh-utils` to copy the resulting binary to the camera and execute it. The test binary is executed with `RUST_LOG=debug` but the `--nocapture` argument is not passed to the binary, so `println!` debugging statements will not be printed to stdout.

To use this test method, change the working directory to the top level directory of the crate to test. Then simply call run the command
`cargo-acap-sdk test --host <host-IP> --user <user> --pass <pass>`.

## Using cargo test runner
Cargo can be configured to invoke a specified runner whenever cargo would normally execute some compiled code such as `cargo run` or `cargo test` [target.\<triple\>.runner](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner).
The basic steps to set up the runner are:
1. Connect an Axis camera to your network and ensure it is accessible.
2. Set the `AXIS_DEVICE_IP` and, optionally, `AXIS_DEVICE_USER` environment variables.
3. Set up an identity based SSH connection to the camera.
   1. Create an ID via `ssh-keygen`
   2. Copy the id to the device via `ssh-copy-id`
4. Add `bin/` to your path e.g. like `export PATH="$(pwd)/bin:$PATH"`

Now, via the [remote-test.sh](../bin/remote-test.sh) script, and the `runner = ["/workspaces/acap-rs/remote-test.sh"]` line in the `.cargo/config.toml`, tests targeting axis device architectures will automatically be copied to the remote camera and executed there.
Run these tests via `cargo test --target aarch64-unknown-linux-gnu`.
This approach is particularly useful if only a specific crate is to be tested, and if `println!` debugging is used.
An example command line invocation might be `cargo test -p acap-vapix --tests -- --nocapture`.

If you want to run tests locally, just make sure you clear the `AXIS_DEVICE_IP` environment variable via `unset AXIS_DEVICE_IP`.

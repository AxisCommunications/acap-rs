Sanitizers are particularly helpful when working with unsafe code which.
Rust has unstable support for compiling programs with a sanitizer[^1], the documentation was insufficient for me to get started.
This is a brief guide on how to build and run a program with AddressSanitizer enabled.
Note that the instructions are specific to an aarch64 device, but adapting them for other architectures should be easy.

Prepare your device and your environment for use with the [custom runner](running-apps-and-tests-on-device.md#using-cargo-test-runner).

Ensure the asan library is in place so that we can load it:
```shell
scp /opt/axis/acapsdk/sysroots/aarch64/usr/lib/libasan.so.8.0.0 $AXIS_DEVICE_USER@$AXIS_DEVICE_IP:/tmp/libasan.so
```

Compile and run the tests:
```shell
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=--sysroot=/opt/axis/acapsdk/sysroots/aarch64 -C link-arg=-lasan -Zsanitizer=address -Z external-clangrt" \
REMOTE_ENV=LD_PRELOAD=/tmp/libasan.so \
cargo +nightly test \
  --package $AXIS_PACKAGE \
  --target aarch64-unknown-linux-gnu
```

[^1]: https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html#sanitizer

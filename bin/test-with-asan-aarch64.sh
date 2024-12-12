#!/usr/bin/env sh
# This script is meant primarily as documentation and you will likely want to adapt it and/or run only parts of it.
set -eux

# Ensure the app is installed so that we have a directory to `scp` the library and the binary to.
cargo-acap-sdk install -- -p ${AXIS_PACKAGE}

# Ensure the library is in place so that we can load it
scp /opt/axis/acapsdk/sysroots/aarch64/usr/lib/libasan.so.8.0.0 ${AXIS_DEVICE_USER}@${AXIS_DEVICE_IP}:/usr/local/packages/${AXIS_PACKAGE}/libasan.so

# Compile, link and run the tests. Relies on `run-on-device.sh` to `scp` and run the binary.
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=--sysroot=/opt/axis/acapsdk/sysroots/aarch64 -C link-arg=-lasan -Zsanitizer=address -Z external-clangrt" \
REMOTE_ENV=LD_PRELOAD=/usr/local/packages/${AXIS_PACKAGE}/libasan.so \
cargo +nightly test --target aarch64-unknown-linux-gnu -p $AXIS_PACKAGE

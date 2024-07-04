#!/usr/bin/env sh
set -eux

curl \
  --output /tmp/rustup-init \
  "https://static.rust-lang.org/rustup/archive/1.26.0/x86_64-unknown-linux-gnu/rustup-init"

echo "0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db /tmp/rustup-init" \
| sha256sum -c -

chmod +x /tmp/rustup-init

/tmp/rustup-init \
  --no-modify-path \
  --no-update-default-toolchain \
  -y

rm /tmp/rustup-init

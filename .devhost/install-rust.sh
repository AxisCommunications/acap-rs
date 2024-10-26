#!/usr/bin/env sh
set -eux

curl \
  --output /tmp/rustup-init \
  "https://static.rust-lang.org/rustup/archive/1.26.0/aarch64-unknown-linux-gnu/rustup-init"

echo "673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800 /tmp/rustup-init" \
| sha256sum -c -

chmod +x /tmp/rustup-init

/tmp/rustup-init \
  --no-modify-path \
  --no-update-default-toolchain \
  -y

rm /tmp/rustup-init

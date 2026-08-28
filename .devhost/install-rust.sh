#!/usr/bin/env sh
set -eux

ARCH=$(uname -m)
case "$ARCH" in
  x86_64)
    URL="https://static.rust-lang.org/rustup/archive/1.26.0/x86_64-unknown-linux-gnu/rustup-init"
    SHA256="0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db"
    ;;
  aarch64)
    URL="https://static.rust-lang.org/rustup/archive/1.26.0/aarch64-unknown-linux-gnu/rustup-init"
    SHA256="673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800"
    ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

curl --output /tmp/rustup-init "$URL"
echo "$SHA256 /tmp/rustup-init" | sha256sum -c -

chmod +x /tmp/rustup-init

/tmp/rustup-init \
  --no-modify-path \
  --no-update-default-toolchain \
  -y

rm /tmp/rustup-init

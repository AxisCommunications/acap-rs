#!/usr/bin/env sh
set -eux
wget "https://static.rust-lang.org/rustup/archive/1.26.0/x86_64-unknown-linux-gnu/rustup-init"
echo "0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db rustup-init" | sha256sum -c -
chmod +x rustup-init

./rustup-init \
  --default-host x86_64-unknown-linux-gnu \
  --default-toolchain $(grep "channel" rust-toolchain.toml | cut -d '"' -f 2) \
  --no-modify-path \
  --profile minimal \
  -y

rm rustup-init
chmod -R a+w $RUSTUP_HOME $CARGO_HOME
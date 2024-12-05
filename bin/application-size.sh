#!/usr/bin/env sh
set -eux

CARGO_TARGET_DIR="targets/baseline" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/baseline --target=x86_64-unknown-linux-gnu -p '*_*'

CARGO_TARGET_DIR="targets/release" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/release --target=x86_64-unknown-linux-gnu -p '*_*' --release

CARGO_TARGET_DIR="targets/stable" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_LTO="true" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/stable --target=x86_64-unknown-linux-gnu -p '*_*' --release

CARGO_TARGET_DIR="targets/unstable" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_LTO="true" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/unstable --target=x86_64-unknown-linux-gnu -p '*_*' --release -Zbuild-std=panic_abort,std -Zbuild-std-features=panic_immediate_abort

CARGO_TARGET_DIR="targets/unstable2" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_LTO="true" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" RUSTFLAGS="-Zlocation-detail=none" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/unstable2 --target=x86_64-unknown-linux-gnu -p '*_*' --release -Zbuild-std=panic_abort,std -Zbuild-std-features=panic_immediate_abort

CARGO_TARGET_DIR="targets/unstable3" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_LTO="true" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/unstable3 --target=x86_64-unknown-linux-gnu -p '*_*' --release -Zbuild-std=panic_abort,std -Zbuild-std-features=panic_immediate_abort

CARGO_TARGET_DIR="targets/unstable4" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_LTO="true" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" \
cargo +nightly build -Zunstable-options --artifact-dir=artifacts/unstable4 --target=x86_64-unknown-linux-gnu -p '*_*' --release -Zbuild-std=panic_abort,std -Zbuild-std-features=panic_immediate_abort,optimize_for_size

#CARGO_TARGET_DIR="targets/unstable5" CARGO_PROFILE_RELEASE_OPT_LEVEL="s" CARGO_PROFILE_RELEASE_STRIP="symbols" CARGO_PROFILE_RELEASE_PANIC="abort" CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" \
#cargo +nightly bloat -Zunstable-options --artifact-dir=artifacts/unstable5 --target=x86_64-unknown-linux-gnu -p '*_*' --release -Zbuild-std=panic_abort,std -Zbuild-std-features=panic_immediate_abort,optimize_for_size


for d in artifacts/*; do
  echo $d
  du -hsc $d/*
done

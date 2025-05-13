## Configuration
## =============

# Parameters
# ----------

# Name of package containing the app to be built.
# Rust does not enforce that the path to the package matches the package name, but
# this makefile does to keep things simple.
export AXIS_PACKAGE ?= hello_world

# The architecture that will be assumed when interacting with the device.
export AXIS_DEVICE_ARCH ?= aarch64

# The IP address of the device to interact with.
export AXIS_DEVICE_IP ?= 192.168.0.90

# The username to use when interacting with the device.
export AXIS_DEVICE_USER ?= root

# The password to use when interacting with the device.
export AXIS_DEVICE_PASS ?= pass

# Reproducible and stable results by default
export SOURCE_DATE_EPOCH ?= 0

# Other
# -----

# Have zero effect by default to prevent accidental changes.
.DEFAULT_GOAL := help

# Delete targets that fail to prevent subsequent attempts incorrectly assuming
# the target is up to date.
.DELETE_ON_ERROR: ;

# Don't remove intermediate files.
.SECONDARY:

# Prevent pesky default rules from creating unexpected dependency graphs.
.SUFFIXES: ;

# Rebuild targets when marking them as phony directly is not enough.
FORCE:;
.PHONY: FORCE

## Verbs
## =====

help:
	@mkhelp $(firstword $(MAKEFILE_LIST))

## Reset <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> to a clean state suitable for development and testing.
reinit:
	RUST_LOG=info device-manager reinit

## Build <AXIS_PACKAGE> for <AXIS_DEVICE_ARCH>
build:
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build \
		--target $(AXIS_DEVICE_ARCH) \
		-- \
		--package $(AXIS_PACKAGE) \
		--profile app

## Discover Axis devices on the local network
##
## Note that this does not work inside a container unless it's running on a Linux host.
discover-devices:
	rs4a discover-devices

## Install <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
install:
	cargo-acap-sdk install \
	-- \
	--profile app

## Remove <AXIS_PACKAGE> from <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
remove:
	cargo-acap-sdk remove

## Start <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
start:
	cargo-acap-sdk start

## Stop <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
stop:
	cargo-acap-sdk stop

## Build and run <AXIS_PACKAGE> directly on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
##
## Prerequisites:
##
## * <AXIS_PACKAGE> is recognized by `cargo-acap-build` as an ACAP app.
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
run:
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE) --profile dev
	acap-ssh-utils patch target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/*.eap
	acap-ssh-utils run-app \
		--environment RUST_LOG=debug \
		--environment RUST_LOG_STYLE=always \
		$(AXIS_PACKAGE)

## Build and execute unit tests for <AXIS_PACKAGE> on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
##
## Prerequisites:
##
## * <AXIS_PACKAGE> is recognized by `cargo-acap-build` as an ACAP app.
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
test:
	# The `scp` command below needs the wildcard to match exactly one file.
	rm -r target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)-*/$(AXIS_PACKAGE) ||:
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE) --profile dev --tests
	acap-ssh-utils patch target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)-*/*.eap
	acap-ssh-utils run-app \
		--environment RUST_LOG=debug \
		--environment RUST_LOG_STYLE=always \
		$(AXIS_PACKAGE) \
		-- \
		--test-threads=1

## Bulk operations
## ---------------

## Install all apps on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
install_all:
	cargo-acap-sdk install \
		-- \
		--package '*_*' \
		--profile app

## Build and execute unit tests for all apps on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
test_all:
	cargo-acap-sdk test \
		-- \
		--package licensekey \
		--package '*_*'

## Checks
## ------

## Run all checks except generated files
check_other: check_build check_docs check_format check_lint check_tests
.PHONY: check_other

## Check that all crates can be built
check_build: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	cargo build \
		--exclude '*_*' \
		--locked \
		--workspace
.PHONY: check_build

## Check that docs can be built
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc \
		--document-private-items \
		--locked \
		--no-deps \
		--workspace
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	RUSTDOCFLAGS="-Dwarnings" cargo doc \
		--document-private-items \
		--locked \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace
.PHONY: check_docs

## Check that the code is formatted correctly
check_format:
	cargo fmt --check
.PHONY: check_format

## Check that generated files are up to date
check_generated_files: Cargo.lock $(patsubst %/,%/src/bindings.rs,$(wildcard crates/*-sys/))
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files

## Check that generated files are up to date, including machine-dependent generated files.
##
## Note that this will likely work only if:
## - The command is run inside the dev container.
## - The name of the repository root is `acap-rs` because this affects the path inside the container.
check_generated_files_container: apps-$(AXIS_DEVICE_ARCH).checksum apps-$(AXIS_DEVICE_ARCH).filesize
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files_container

## Check that the code is free of lints
check_lint:
	cargo clippy \
		--all-targets \
		--locked \
		--no-deps \
		--workspace \
		-- \
		-Dwarnings
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo clippy \
		--all-targets \
		--locked \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace \
		-- \
		-Dwarnings
.PHONY: check_lint

## Check that risky FFI patterns are sound using miri
check_miri:
	rustup +nightly component add miri
	cargo +nightly miri test \
		--package ffi_patterns \
		--target aarch64-unknown-linux-gnu \
		--target thumbv7neon-unknown-linux-gnueabihf

## _
check_tests:
	cargo test \
		--exclude '*_*' \
		--exclude '*-sys' \
		--exclude axevent \
		--exclude axstorage \
		--exclude bbox \
		--exclude licensekey \
		--exclude mdb \
		--locked \
		--workspace
.PHONY: check_tests

## Fixes
## -----

## Attempt to fix formatting automatically
fix_format:
	find apps/axserialport_example crates/axserialport -type f -name '*.rs' \
 	| xargs rustfmt \
 		--config imports_granularity=Crate \
 		--config group_imports=StdExternalCrate \
 		--edition 2021
	cargo fmt
.PHONY: fix_format

## Attempt to fix lints automatically
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

Cargo.lock: FORCE
	cargo metadata > /dev/null

.devhost/constraints.txt: .devhost/requirements.txt
	pip-compile \
		--allow-unsafe \
		--no-header \
		--quiet \
		--strip-extras \
		--output-file $@ \
		$^

apps-$(AXIS_DEVICE_ARCH).checksum: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	find target-$(AXIS_DEVICE_ARCH)/acap/ -name '*.eap' | LC_ALL=C sort | xargs shasum > $@

apps-$(AXIS_DEVICE_ARCH).filesize: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	find target-$(AXIS_DEVICE_ARCH)/acap/ -name '*.eap' | LC_ALL=C sort | xargs du --apparent-size > $@

crates/%-sys/src/bindings.rs: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	cp --archive $(firstword $(wildcard target-$(AXIS_DEVICE_ARCH)/*/*/build/$*-sys-*/out/bindings.rs)) $@

target-$(AXIS_DEVICE_ARCH)/acap/_envoy:
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build \
		--target $(AXIS_DEVICE_ARCH) \
		-- \
		--package '*_*' \
		--profile dev \
		--locked
	touch $@

.PHONY: target-$(AXIS_DEVICE_ARCH)/acap/_envoy

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

ACAP_BUILD = . /opt/axis/acapsdk/$(ENVIRONMENT_SETUP) && cd $(@D) && acap-build --build no-build .

# It doesn't matter which SDK is sourced for installing, but using a wildcard would fail since there are multiple in the container.
EAP_INSTALL = cd $(CURDIR)/target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/ \
&& . /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux && eap-install.sh $(AXIS_DEVICE_IP) $(AXIS_DEVICE_PASS) $@


## Verbs
## =====

help:
	@mkhelp print_docs $(firstword $(MAKEFILE_LIST)) help

## Reset <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> to a clean state suitable for development and testing.
reinit:
	RUST_LOG=info device-manager reinit

## Build <AXIS_PACKAGE> for <AXIS_DEVICE_ARCH>
build: apps/$(AXIS_PACKAGE)/LICENSE
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE)

## Install <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
install:
	@ $(EAP_INSTALL) \
	| grep -v '^to start your application type$$' \
	| grep -v '^  eap-install.sh start$$'

## Remove <AXIS_PACKAGE> from <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
remove:
	@ $(EAP_INSTALL)

## Start <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
start:
	@ $(EAP_INSTALL) \
	| grep -v '^to stop your application type$$' \
	| grep -v '^  eap-install.sh stop$$'

## Stop <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
stop:
	@ $(EAP_INSTALL)

## Build and run <AXIS_PACKAGE> directly on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
##
## Forwards the following environment variables to the remote process:
##
## * `RUST_LOG`
## * `RUST_LOG_STYLE`
##
## Prerequisites:
##
## * <AXIS_PACKAGE> is recognized by `cargo-acap-build` as an ACAP app.
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
## * The device is added to `knownhosts`.
run:
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE)
	acap-ssh-utils patch target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/*.eap
	acap-ssh-utils run-app \
		--environment RUST_LOG=debug \
		--environment RUST_LOG_STYLE=always \
		$(AXIS_PACKAGE)

## Build and execute unit tests for <AXIS_PACKAGE> on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
##
## Forwards the following environment variables to the remote process:
##
## * `RUST_LOG`
## * `RUST_LOG_STYLE`
##
## Prerequisites:
##
## * <AXIS_PACKAGE> is recognized by `cargo-acap-build` as an ACAP app.
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
## * The device is added to `knownhosts`.
test:
	# The `scp` command below needs the wildcard to match exactly one file.
	rm -r target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)-*/$(AXIS_PACKAGE) ||:
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE) --tests
	acap-ssh-utils patch target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)-*/*.eap
	acap-ssh-utils run-app \
		--environment RUST_LOG=debug \
		--environment RUST_LOG_STYLE=always \
		$(AXIS_PACKAGE) \
		-- \
		--test-threads=1

## Checks
## ------

## Run all other checks
check_all: check_build check_docs check_format check_lint check_tests check_generated_files
.PHONY: check_all

## Check that all crates can be built
check_build: $(patsubst %/,%/LICENSE,$(wildcard apps/*/))
	cargo build \
		--exclude consume_analytics_metadata \
		--exclude axevent \
		--exclude axevent-sys \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--exclude mdb \
		--exclude mdb-sys \
		--exclude send_event \
		--workspace
	cargo-acap-build \
		--target aarch64 \
		-- \
		--exclude acap-ssh-utils \
		--exclude cargo-acap-build \
		--exclude device-manager \
		--workspace

.PHONY: check_build

## Check that docs can be built
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc
	RUSTDOCFLAGS="-Dwarnings" cargo doc \
		--document-private-items \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace
.PHONY: check_docs

## Check that the code is formatted correctly
check_format:
	cargo fmt --check
.PHONY: check_format

## Check that generated files are up to date
check_generated_files: $(patsubst %/,%/src/bindings.rs,$(wildcard crates/*-sys/))
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files

## Check that the code is free of lints
check_lint:
	cargo clippy \
		--all-targets \
		--no-deps \
		--exclude consume_analytics_metadata \
		--exclude axevent \
		--exclude axevent-sys \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--exclude mdb \
		--exclude mdb-sys \
		--exclude send_event \
		--workspace -- -Dwarnings
	cargo clippy \
		--all-targets \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace -- -Dwarnings
.PHONY: check_lint

## _
check_tests:
	cargo test \
			--exclude consume_analytics_metadata \
			--exclude axevent \
			--exclude axevent-sys \
			--exclude licensekey \
			--exclude licensekey-sys \
			--exclude licensekey_handler \
			--exclude mdb \
			--exclude mdb-sys \
			--exclude send_event \
			--workspace
.PHONY: check_tests

## Fixes
## -----

## Attempt to fix formatting automatically
fix_format:
	cargo fmt
.PHONY: fix_format

## Attempt to fix lints automatically
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

.devhost/constraints.txt: .devhost/requirements.txt
	pip-compile \
		--allow-unsafe \
		--no-header \
		--quiet \
		--strip-extras \
		--output-file $@ \
		$^

# TODO: Find a convenient way to integrate this with cargo-acap-build
apps/%/LICENSE: apps/%/Cargo.toml about.hbs
	cargo-about generate \
		--manifest-path apps/$*/Cargo.toml \
		--output-file $@ \
		about.hbs

crates/%-sys/src/bindings.rs: FORCE
	cp $(firstword $(wildcard target/*/*/build/$*-sys-*/out/bindings.rs)) $@

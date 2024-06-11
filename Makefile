## Configuration
## =============

# Parameters
# ----------

# Name of package containing the app to be built.
# Rust does not enforce that the path to the package matches the package name, but
# this makefile does to keep things simple.
PACKAGE ?= hello_world

# The architecture that will be assumed when interacting with the device.
ARCH ?= aarch64

# The IP address of the device to interact with.
DEVICE_IP ?= 192.168.0.90

# The password to use when interacting with the device.
PASS ?= pass

# Other
# -----

# Have zero effect by default to prevent accidental changes.
.DEFAULT_GOAL := help

# Delete targets that fail to prevent subsequent attempts incorrectly assuming
# the target is up to date.
.DELETE_ON_ERROR: ;

# Prevent pesky default rules from creating unexpected dependency graphs.
.SUFFIXES: ;

# Rebuild targets when marking them as phony directly is not enough.
FORCE:;
.PHONY: FORCE

CARGO_HOME ?= $(HOME)/.cargo

EAP_INSTALL = cd ${CURDIR}/target/$(ARCH)/$(PACKAGE)/ \
&& cargo-acap-sdk containerize --docker-file ${CURDIR}/Dockerfile -- \
sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(DEVICE_IP) $(PASS) $@"

## Verbs
## =====

help:
	@mkhelp print_docs $(firstword $(MAKEFILE_LIST)) help

## Build <PACKAGE> for all architectures
build:
	cargo-acap-sdk build \
		--docker-file $(CURDIR)/Dockerfile \
		--package $(PACKAGE) \
		--target aarch64 \
		--target armv7hf



## Install <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
install:
	cargo-acap-sdk install \
		--docker-file $(CURDIR)/Dockerfile \
		--username root \
		--password $(PASS) \
		--target $(ARCH) \
		--address $(DEVICE_IP) \
		--package $(PACKAGE)

## Remove <PACKAGE> from <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
remove:
	@ $(EAP_INSTALL)

## Start <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
start:
	@ # Don't match the line endings because docker replace LF with CR + LF when given `--tty`
	@ $(EAP_INSTALL) \
	| grep -v '^to stop your application type' \
	| grep -v '^  eap-install.sh stop'

## Stop <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
stop:
	@ $(EAP_INSTALL)

## Build and run <PACKAGE> directly on <DEVICE_IP> assuming architecture <ARCH>
##
## Prerequisites:
##
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
## * The device is added to `knownhosts`.
run:
	cargo-acap-sdk run \
		--docker-file $(CURDIR)/Dockerfile \
		--username root \
		--password $(PASS) \
		--target $(ARCH) \
		--address $(DEVICE_IP) \
		--package $(PACKAGE)

## Build and execute unit and integration tests for <PACKAGE> on <DEVICE_IP> assuming architecture <ARCH>
##
## Prerequisites:
##
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
## * The device is added to `knownhosts`.
test:
	cargo-acap-sdk test \
		--docker-file $(CURDIR)/Dockerfile \
		--username root \
		--password $(PASS) \
		--target $(ARCH) \
		--address $(DEVICE_IP) \
		--package $(PACKAGE)

# TODO: Find a better way to test
# Quick and dirty way to ensure all commands work in an container and on host
exercise_cargo_acap_sdk:
	mkdir -p venv/tmp
	# Clean build.
	rm -r target/
	RUST_LOG=debug cargo run -p cargo-acap-sdk -- build --docker-file $(CURDIR)/Dockerfile -p $(PACKAGE) > venv/tmp/build_cold.log 2>&1
	# Incremental builds.
	RUST_LOG=debug cargo run -p cargo-acap-sdk -- build --docker-file $(CURDIR)/Dockerfile -p $(PACKAGE) > venv/tmp/build_warm.log 2>&1
	RUST_LOG=debug cargo run -p cargo-acap-sdk -- install --docker-file $(CURDIR)/Dockerfile --address $(DEVICE_IP) --password $(PASS) --target $(ARCH) -p $(PACKAGE) > venv/tmp/install.log 2>&1
	RUST_LOG=debug cargo run -p cargo-acap-sdk -- run  --docker-file $(CURDIR)/Dockerfile --address $(DEVICE_IP) --password $(PASS) --target $(ARCH) -p $(PACKAGE) > venv/tmp/run.log 2>&1
	RUST_LOG=debug cargo run -p cargo-acap-sdk -- test --docker-file $(CURDIR)/Dockerfile --address $(DEVICE_IP) --password $(PASS) --target $(ARCH) -p $(PACKAGE) > venv/tmp/test.log 2>&1
	# The above should ensure that the docker image and the exe are available for the below
	#
	# Use the exe directly because `cargo run` will set `LD_LIBRARY_PATH` causing `acap-build` to fail.
	# Don't test commands that deeply because
	# 1. they will probably not be used from within a container, and
	# 2. they make it more complex to build and run the docker image
	docker run \
		--env RUST_LOG=debug \
		--rm \
		--user $(shell id -u):$(shell id -g) \
		--volume $$(realpath $(CARGO_HOME)):/usr/local/cargo \
		--volume ${CURDIR}:${CURDIR} \
		--workdir ${CURDIR} \
		acap-rs ./target/debug/cargo-acap-sdk build --no-docker -p $(PACKAGE) \
	> venv/tmp/build_warm_custom.log 2>&1

## Install development dependencies
sync_env:
	cargo install --root venv --target-dir $(CURDIR)/target --path $(CURDIR)/crates/acap-ssh-utils
	cargo install --root venv --target-dir $(CURDIR)/target --path $(CURDIR)/crates/cargo-acap-sdk
	PIP_CONSTRAINT=constraints.txt pip install --requirement requirements.txt

## Checks
## ------

## Run all other checks
check_all: check_build check_docs check_format check_lint check_tests check_generated_files
.PHONY: check_all

## Check that all crates can be built
check_build:
	cargo build \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--workspace
	cargo-acap-sdk containerize --docker-file $(CURDIR)/Dockerfile -- cargo build \
		--exclude acap-ssh-utils \
		--target aarch64-unknown-linux-gnu \
		--workspace

.PHONY: check_build

## Check that docs can be built
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc
	cargo-acap-sdk containerize --docker-file $(CURDIR)/Dockerfile --docker-env RUSTFLAGS="-Dwarnings" -- cargo doc \
		--document-private-items \
		--exclude acap-ssh-utils \
		--exclude cargo-acap-sdk \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace
.PHONY: check_docs

## _
check_format:
	cargo fmt --check
.PHONY: check_format

## Check that generated files are up to date
check_generated_files: $(patsubst %/,%/src/bindings.rs,$(wildcard crates/*-sys/))
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files


## _
check_lint:
	RUSTFLAGS="-Dwarnings" cargo clippy \
		--all-targets \
		--no-deps \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--workspace
	cargo-acap-sdk containerize --docker-file $(CURDIR)/Dockerfile --docker-env RUSTFLAGS="-Dwarnings" -- cargo clippy \
		--all-targets \
		--exclude acap-ssh-utils \
		--exclude cargo-acap-sdk \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace
.PHONY: check_lint

## _
check_tests:
	cargo test \
			--exclude licensekey \
			--exclude licensekey-sys \
			--exclude licensekey_handler \
			--workspace
.PHONY: check_tests

## Fixes
## -----

## _
fix_format:
	cargo fmt
.PHONY: fix_format

## _
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

constraints.txt: requirements.txt
	pip-compile \
		--allow-unsafe \
		--no-header \
		--quiet \
		--strip-extras \
		--output-file $@ \
		$^

crates/%-sys/src/bindings.rs: FORCE
	cp $(firstword $(wildcard target/*/*/build/$*-sys-*/out/bindings.rs)) $@

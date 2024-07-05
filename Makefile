## Configuration
## =============

# Parameters
# ----------

# Name of package containing the app to be built.
# Rust does not enforce that the path to the package matches the package name, but
# this makefile does to keep things simple.
AXIS_PACKAGE ?= hello_world

# The architecture that will be assumed when interacting with the device.
AXIS_DEVICE_ARCH ?= aarch64

# The IP address of the device to interact with.
AXIS_DEVICE_IP ?= 192.168.0.90

# The password to use when interacting with the device.
AXIS_DEVICE_PASS ?= pass

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

# Use the current environment when already in a container.
ifeq (0, $(shell test -e /.dockerenv; echo $$?))

ACAP_BUILD = . /opt/axis/acapsdk/$(ENVIRONMENT_SETUP) && cd $(@D) && acap-build --build no-build .

CROSS := cargo

# It doesn't matter which SDK is sourced for installing, but using a wildcard would fail since there are multiple in the container.
EAP_INSTALL = cd $(CURDIR)/target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/ \
&& . /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux && eap-install.sh $(AXIS_DEVICE_IP) $(AXIS_DEVICE_PASS) $@

# Use a containerized environment when running on host.
else

# Bare minimum to make the output from the container available on host with correct permissions.
DOCKER_RUN = docker run \
--volume ${CURDIR}/target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/:/opt/app \
--user $(shell id -u):$(shell id -g) \
axisecp/acap-native-sdk:1.14-$(AXIS_DEVICE_ARCH)-ubuntu22.04

ACAP_BUILD = $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && acap-build --build no-build ."

CROSS := cross

EAP_INSTALL = $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(AXIS_DEVICE_IP) $(AXIS_DEVICE_PASS) $@"

endif


## Verbs
## =====

help:
	@mkhelp print_docs $(firstword $(MAKEFILE_LIST)) help

## Build <AXIS_PACKAGE> for <AXIS_DEVICE_ARCH>
build: target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/_envoy
	mkdir -p target/acap
	cp $(patsubst %/_envoy,%/*.eap,$^) target/acap

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
## * The app is installed on the device.
## * The app is stopped.
## * The device has SSH enabled the ssh user root configured.
run: target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)/$(AXIS_PACKAGE)
	scp $< root@$(AXIS_DEVICE_IP):/usr/local/packages/$(AXIS_PACKAGE)/$(AXIS_PACKAGE)
	ssh root@$(AXIS_DEVICE_IP) \
		"cd /usr/local/packages/$(AXIS_PACKAGE) && su - acap-$(AXIS_PACKAGE) -s /bin/sh --preserve-environment -c '$(if $(RUST_LOG_STYLE),RUST_LOG_STYLE=$(RUST_LOG_STYLE) )$(if $(RUST_LOG),RUST_LOG=$(RUST_LOG) )./$(AXIS_PACKAGE)'"

## Install development dependencies
sync_env: venv/bin/npm
	cargo install --root venv --target-dir $(CURDIR)/target cross
	PIP_CONSTRAINT=constraints.txt pip install --requirement requirements.txt
	npm install -g @devcontainers/cli@0.65.0

## Checks
## ------

## Run all other checks
check_all: check_build check_docs check_format check_lint check_tests check_generated_files
.PHONY: check_all

## Check that all crates can be built
check_build: target/aarch64/$(AXIS_PACKAGE)/_envoy target/armv7hf/$(AXIS_PACKAGE)/_envoy
	cargo build \
		--exclude consume_analytics_metadata \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--exclude mdb \
		--exclude mdb-sys \
		--workspace
	$(CROSS) build \
		--target aarch64-unknown-linux-gnu \
		--workspace

.PHONY: check_build

## Check that docs can be built
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc
	RUSTDOCFLAGS="-Dwarnings" $(CROSS) doc \
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
	RUSTFLAGS="-Dwarnings" cargo clippy \
		--all-targets \
		--no-deps \
		--exclude consume_analytics_metadata \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--exclude mdb \
		--exclude mdb-sys \
		--workspace
	RUSTFLAGS="-Dwarnings" $(CROSS) clippy \
		--all-targets \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace
.PHONY: check_lint

## _
check_tests:
	cargo test \
			--exclude consume_analytics_metadata \
			--exclude licensekey \
			--exclude licensekey-sys \
			--exclude licensekey_handler \
			--exclude mdb \
			--exclude mdb-sys \
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

# Stage the files that will be packaged outside the source tree to avoid
# * cluttering the source tree and `.gitignore` with build artifacts, and
# * having the same file be built for different targets at different times.
# Use the `_envoy` file as a target because
# * `.DELETE_ON_ERROR` does not work for directories, and
# * the name of the `.eap` file is annoying to predict.
# When building for all targets using a single image we cannot rely on wildcard matching.
target/aarch64/$(AXIS_PACKAGE)/_envoy: ENVIRONMENT_SETUP=environment-setup-cortexa53-crypto-poky-linux
target/armv7hf/$(AXIS_PACKAGE)/_envoy: ENVIRONMENT_SETUP=environment-setup-cortexa9hf-neon-poky-linux-gnueabi
target/%/$(AXIS_PACKAGE)/_envoy: AXIS_DEVICE_ARCH=$*
target/%/$(AXIS_PACKAGE)/_envoy: target/%/$(AXIS_PACKAGE)/lib target/%/$(AXIS_PACKAGE)/html target/%/$(AXIS_PACKAGE)/$(AXIS_PACKAGE) target/%/$(AXIS_PACKAGE)/manifest.json target/%/$(AXIS_PACKAGE)/LICENSE
	$(ACAP_BUILD)
	touch $@

target/%/$(AXIS_PACKAGE)/html: FORCE
	mkdir -p $(dir $@)
	if [ -d $@ ]; then rm -r $@; fi
	if [ -d apps/$(AXIS_PACKAGE)/html ]; then cp -r apps/$(AXIS_PACKAGE)/html $@; fi

target/%/$(AXIS_PACKAGE)/lib: FORCE
	mkdir -p $(dir $@)
	if [ -d $@ ]; then rm -r $@; fi
	if [ -d apps/$(AXIS_PACKAGE)/lib ]; then cp -r apps/$(AXIS_PACKAGE)/lib $@; fi

target/%/$(AXIS_PACKAGE)/manifest.json: apps/$(AXIS_PACKAGE)/manifest.json
	mkdir -p $(dir $@)
	cp $< $@

target/%/$(AXIS_PACKAGE)/LICENSE: apps/$(AXIS_PACKAGE)/LICENSE
	mkdir -p $(dir $@)
	cp $< $@

# The target triple and the name of the docker image do not match, so
# at some point we need to map one to the other. It might as well be here.
target/aarch64/$(AXIS_PACKAGE)/$(AXIS_PACKAGE): target/aarch64-unknown-linux-gnu/release/$(AXIS_PACKAGE)
	mkdir -p $(dir $@)
	cp $< $@

target/armv7hf/$(AXIS_PACKAGE)/$(AXIS_PACKAGE): target/thumbv7neon-unknown-linux-gnueabihf/release/$(AXIS_PACKAGE)
	mkdir -p $(dir $@)
	cp $< $@

# Always rebuild the executable because configuring accurate cache invalidation is annoying.
target/%/release/$(AXIS_PACKAGE): FORCE
	$(CROSS) -v build --release --target $* --package $(AXIS_PACKAGE)
	touch $@ # This is a hack to make the `_envoy` target above always build


venv/bin/npm: venv/downloads/node-v18.16.1-linux-x64.tar.gz
	tar -xf "$<" --strip-components 1 -C venv

venv/downloads/node-v18.16.1-linux-x64.tar.gz:
	mkdir -p $(@D)
	curl -L -o "$@" "https://nodejs.org/dist/v18.16.1/node-v18.16.1-linux-x64.tar.gz"
	echo "59582f51570d0857de6333620323bdeee5ae36107318f86ce5eca24747cabf5b  $@" | sha256sum -c -

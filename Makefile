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

# Don't remove intermediate files.
.SECONDARY:

# Prevent pesky default rules from creating unexpected dependency graphs.
.SUFFIXES: ;

# Rebuild targets when marking them as phony directly is not enough.
FORCE:;
.PHONY: FORCE

DOCKER_RUN = docker run \
--volume ${CURDIR}/target/$(ARCH)/$(PACKAGE)/:/opt/app \
--user $(shell id -u):$(shell id -g) \
axisecp/acap-native-sdk:1.12-$(ARCH)-ubuntu22.04


## Verbs
## =====

help:
	@mkhelp print_docs $(firstword $(MAKEFILE_LIST)) help

## Build <PACKAGE> for all architectures
build: target/aarch64/$(PACKAGE)/_envoy target/armv7hf/$(PACKAGE)/_envoy
	mkdir -p target/acap
	cp $(patsubst %/_envoy,%/*.eap,$^) target/acap

## Install <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
install:
	@ $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(DEVICE_IP) $(PASS) install" \
	| grep -v '^to start your application type$$' \
	| grep -v '^  eap-install.sh start$$'

## Remove <PACKAGE> from <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
remove:
	@ $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(DEVICE_IP) $(PASS) remove"

## Start <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
start:
	@ $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(DEVICE_IP) $(PASS) start" \
	| grep -v '^to stop your application type$$' \
	| grep -v '^  eap-install.sh stop$$'

## Stop <PACKAGE> on <DEVICE_IP> using password <PASS> and assuming architecture <ARCH>
stop:
	@ $(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && eap-install.sh $(DEVICE_IP) $(PASS) stop"

## Build and run <PACKAGE> directly on <DEVICE_IP> assuming architecture <ARCH>
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
run: target/$(ARCH)/$(PACKAGE)/$(PACKAGE)
	scp $< root@$(DEVICE_IP):/usr/local/packages/$(PACKAGE)/$(PACKAGE)
	ssh root@$(DEVICE_IP) \
		"cd /usr/local/packages/$(PACKAGE) && su - acap-$(PACKAGE) -s /bin/sh --preserve-environment -c '$(if $(RUST_LOG_STYLE),RUST_LOG_STYLE=$(RUST_LOG_STYLE) )$(if $(RUST_LOG),RUST_LOG=$(RUST_LOG) )./$(PACKAGE)'"

## Install development dependencies
sync_env:
	cargo install --root venv --target-dir $(CURDIR)/target cross
	PIP_CONSTRAINT=constraints.txt pip install --requirement requirements.txt

## Checks
## ------

## Run all other checks
check_all: check_build check_docs check_format check_lint check_tests check_generated_files
.PHONY: check_all

## Check that all crates can be built
check_build: target/aarch64/$(PACKAGE)/_envoy target/armv7hf/$(PACKAGE)/_envoy
	cargo build \
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--workspace
	cross build \
		--target aarch64-unknown-linux-gnu \
		--workspace

.PHONY: check_build

## Check that docs can be built
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc
	RUSTDOCFLAGS="-Dwarnings" cross doc \
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
		--exclude licensekey \
		--exclude licensekey-sys \
		--exclude licensekey_handler \
		--workspace
	RUSTFLAGS="-Dwarnings" cross clippy \
		--all-targets \
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
target/aarch64/$(PACKAGE)/_envoy: ENVIRONMENT_SETUP=environment-setup-cortexa53-crypto-poky-linux
target/armv7hf/$(PACKAGE)/_envoy: ENVIRONMENT_SETUP=environment-setup-cortexa9hf-neon-poky-linux-gnueabi
target/%/$(PACKAGE)/_envoy: ARCH=$*
target/%/$(PACKAGE)/_envoy: target/%/$(PACKAGE)/lib target/%/$(PACKAGE)/html target/%/$(PACKAGE)/$(PACKAGE) target/%/$(PACKAGE)/manifest.json target/%/$(PACKAGE)/LICENSE
ifeq (0, $(shell test -e /.dockerenv; echo $$?))
	. /opt/axis/acapsdk/$(ENVIRONMENT_SETUP) && cd $(@D) && acap-build --build no-build .
else
	$(DOCKER_RUN) sh -c ". /opt/axis/acapsdk/environment-setup-* && acap-build --build no-build ."
endif
	touch $@

target/%/$(PACKAGE)/html: FORCE
	mkdir -p $(dir $@)
	if [ -d $@ ]; then rm -r $@; fi
	if [ -d apps/$(PACKAGE)/html ]; then cp -r apps/$(PACKAGE)/html $@; fi

target/%/$(PACKAGE)/lib: FORCE
	mkdir -p $(dir $@)
	if [ -d $@ ]; then rm -r $@; fi
	if [ -d apps/$(PACKAGE)/lib ]; then cp -r apps/$(PACKAGE)/lib $@; fi

target/%/$(PACKAGE)/manifest.json: apps/$(PACKAGE)/manifest.json
	mkdir -p $(dir $@)
	cp $< $@

target/%/$(PACKAGE)/LICENSE: apps/$(PACKAGE)/LICENSE
	mkdir -p $(dir $@)
	cp $< $@

# The target triple and the name of the docker image do not match, so
# at some point we need to map one to the other. It might as well be here.
target/aarch64/$(PACKAGE)/$(PACKAGE): target/aarch64-unknown-linux-gnu/release/$(PACKAGE)
	mkdir -p $(dir $@)
	cp $< $@

target/armv7hf/$(PACKAGE)/$(PACKAGE): target/thumbv7neon-unknown-linux-gnueabihf/release/$(PACKAGE)
	mkdir -p $(dir $@)
	cp $< $@

# Always rebuild the executable because configuring accurate cache invalidation is annoying.
target/%/release/$(PACKAGE): FORCE
ifeq (0, $(shell test -e /.dockerenv; echo $$?))
	cargo -v build --release --target $* --package $(PACKAGE)
else
	cross -v build --release --target $* --package $(PACKAGE)
endif
	touch $@ # This is a hack to make the `_envoy` target above always build

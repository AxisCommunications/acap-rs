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
build: apps/$(AXIS_PACKAGE)/LICENSE
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build \
		--target $(AXIS_DEVICE_ARCH) \
		-- \
		--package $(AXIS_PACKAGE) \
		--profile app

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
run: apps/$(AXIS_PACKAGE)/LICENSE
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE)
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
test: apps/$(AXIS_PACKAGE)/LICENSE
	# The `scp` command below needs the wildcard to match exactly one file.
	rm -r target/$(AXIS_DEVICE_ARCH)/$(AXIS_PACKAGE)-*/$(AXIS_PACKAGE) ||:
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	cargo-acap-build --target $(AXIS_DEVICE_ARCH) -- -p $(AXIS_PACKAGE) --tests
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
install_all: $(patsubst %/,%/LICENSE,$(wildcard apps/*/))
	cargo-acap-sdk install \
		-- \
		--package '*_*' \
		--profile app

## Build and execute unit tests for all apps on <AXIS_DEVICE_IP> assuming architecture <AXIS_DEVICE_ARCH>
test_all: $(patsubst %/,%/LICENSE,$(wildcard apps/*/))
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

# TODO: Find a convenient way to integrate this with cargo-acap-build
apps/%/LICENSE: apps/%/Cargo.toml about.hbs
	cargo-about generate \
		--fail \
		--manifest-path apps/$*/Cargo.toml \
		--output-file $@ \
		about.hbs

apps-$(AXIS_DEVICE_ARCH).checksum: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	find target-$(AXIS_DEVICE_ARCH)/acap/ -name '*.eap' | LC_ALL=C sort | xargs shasum > $@

apps-$(AXIS_DEVICE_ARCH).filesize: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	find target-$(AXIS_DEVICE_ARCH)/acap/ -name '*.eap' | LC_ALL=C sort | xargs du --apparent-size > $@

crates/%-sys/src/bindings.rs: target-$(AXIS_DEVICE_ARCH)/acap/_envoy
	cp --archive $(firstword $(wildcard target-$(AXIS_DEVICE_ARCH)/*/*/build/$*-sys-*/out/bindings.rs)) $@

target-$(AXIS_DEVICE_ARCH)/acap/_envoy: target/debug/cargo-acap-build $(patsubst %/,%/LICENSE,$(wildcard apps/*/))
	rm -r $(@D) ||:
	cargo build --bin cargo-acap-build
	ACAP_BUILD_RUST=1 \
	CARGO_TARGET_DIR=target-$(AXIS_DEVICE_ARCH) \
	./target/debug/cargo-acap-build \
		--target $(AXIS_DEVICE_ARCH) \
		-- \
		--package '*_*' \
		--locked
	touch $@

.PHONY: target-$(AXIS_DEVICE_ARCH)/acap/_envoy

target/debug/acap-build:
	cargo build --bin acap-build

.PHONY: target/debug/acap-build

target/debug/cargo-acap-build:
	cargo build --bin cargo-acap-build

.PHONY: target/debug/cargo-acap-build

APPS := \
	utility-libraries/openssl_curl_example \
	using-opencv \
	web-server \
	axevent/send_event \
	axevent/subscribe_to_event \
	axevent/subscribe_to_events \
	axoverlay \
	axparameter \
	axserialport \
	axstorage \
	curl-openssl \
	hello-world \
	licensekey \
	reproducible-package \
	shell-script-example \
	utility-libraries/custom_lib_example \
	vapix \
	vdo-opencl-filtering \
	web-server-using-fastcgi \
	bounding-box \
	message-broker/consume-scene-metadata \
	remote-debug-example


ARCH ?= armv7hf
VERSION ?= 12.0.0
UBUNTU_VERSION ?= 24.04
REPO ?= axisecp
SDK ?= acap-native-sdk

build/_envoy: build/py/_envoy build/rs/_envoy

build/acap-native-sdk-examples/_envoy:
	mkdir -p $(@D)
	git clone https://github.com/AxisCommunications/acap-native-sdk-examples.git $(@D)
	cd $(@D) && git checkout 9b00b2fdf23672f8910421653706572201c2ed8b
	touch $@

docker/rs: target/debug/acap-build
docker/%: docker/%.Dockerfile
	docker build --build-arg ARCH --tag $(REPO)/$*-$(SDK):$(VERSION)-$(ARCH)-ubuntu$(UBUNTU_VERSION) -f docker/$*.Dockerfile .
	touch $@

build/py/_envoy: $(patsubst %,build/py/%/_envoy,$(APPS))
	touch $@

build/py/web-server/_envoy: build/acap-native-sdk-examples/_envoy docker/py
	# Copy source
	rm -r $(@D) ||:
	mkdir -p $(dir $(@D))
	cp -r build/acap-native-sdk-examples/web-server $(@D)
	# Build app
	cd $(@D) \
	&& docker build --build-arg ARCH=$(ARCH) --build-arg SDK=py-$(SDK) --tag web-server . \
	&& docker cp $$(docker create web-server):/opt/monkey/examples ./build
	touch $@

build/py/reproducible-package/_envoy: TIMESTAMP=--build-arg TIMESTAMP=0
build/py/%/_envoy: build/acap-native-sdk-examples/_envoy docker/py
	# Copy source
	rm -r $(@D) ||:
	mkdir -p $(dir $(@D))
	cp -r build/acap-native-sdk-examples/$* $(@D)
	# Build app
	cd $(@D) \
	&& docker build $(TIMESTAMP) --build-arg ARCH=$(ARCH) --build-arg SDK=py-$(SDK) --tag $(notdir $*) . \
	&& docker cp $$(docker create $(notdir $*)):/opt/app ./build
	touch $@

build/rs/_envoy: $(patsubst %,build/rs/%/_envoy,$(APPS))
	touch $@

build/rs/web-server/_envoy: build/acap-native-sdk-examples/_envoy docker/rs
	# Copy source
	rm -r $(@D) ||:
	mkdir -p $(dir $(@D))
	cp -r build/acap-native-sdk-examples/web-server $(@D)
	# Build app
	cd $(@D) \
	&& docker build --build-arg ARCH=$(ARCH) --build-arg SDK=rs-$(SDK) --tag web-server . \
	&& docker cp $$(docker create web-server):/opt/monkey/examples ./build
	touch $@

build/rs/reproducible-package/_envoy: TIMESTAMP=--build-arg TIMESTAMP=0
build/rs/%/_envoy: build/acap-native-sdk-examples/_envoy docker/rs
	# Copy source
	rm -r $(@D) ||:
	mkdir -p $(dir $(@D))
	cp -r build/acap-native-sdk-examples/$* $(@D)
	# Build app
	cd $(@D) \
	&& docker build $(TIMESTAMP) --build-arg ARCH=$(ARCH) --build-arg SDK=rs-$(SDK) --tag $(notdir $*) . \
	&& docker cp $$(docker create $(notdir $*)):/opt/app ./build
	touch $@


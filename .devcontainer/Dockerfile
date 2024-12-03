ARG REPO=axisecp
ARG SDK=acap-native-sdk
ARG UBUNTU_VERSION=22.04
# Keep in sync with `install-sdk.sh`.
ARG VERSION=1.3
ARG BASE_IMAGE=debian:bookworm-20240423

FROM --platform=linux/amd64 ${REPO}/${SDK}:${VERSION}-aarch64-ubuntu${UBUNTU_VERSION} AS sdk-aarch64
FROM --platform=linux/amd64 ${REPO}/${SDK}:${VERSION}-armv7hf-ubuntu${UBUNTU_VERSION} AS sdk-armv7hf
FROM --platform=linux/amd64 ${BASE_IMAGE}

COPY --from=sdk-aarch64 /opt/axis/acapsdk/axis-acap-manifest-tools /opt/axis/acapsdk/axis-acap-manifest-tools
COPY --from=sdk-aarch64 /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux
COPY --from=sdk-armv7hf /opt/axis/acapsdk/environment-setup-cortexa9hf-neon-poky-linux-gnueabi /opt/axis/acapsdk/environment-setup-cortexa9hf-neon-poky-linux-gnueabi
COPY --from=sdk-aarch64 /opt/axis/acapsdk/sysroots/aarch64 /opt/axis/acapsdk/sysroots/aarch64
COPY --from=sdk-armv7hf /opt/axis/acapsdk/sysroots/armv7hf /opt/axis/acapsdk/sysroots/armv7hf
COPY --from=sdk-aarch64 /opt/axis/acapsdk/sysroots/x86_64-pokysdk-linux /opt/axis/acapsdk/sysroots/x86_64-pokysdk-linux

ENV \
    SYSROOT_AARCH64=/opt/axis/acapsdk/sysroots/aarch64 \
    SYSROOT_ARMV7HF=/opt/axis/acapsdk/sysroots/armv7hf
# The above makes the below easier to read
ENV \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-args=--sysroot=${SYSROOT_AARCH64}" \
    CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc" \
    CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++" \
    CARGO_TARGET_THUMBV7NEON_UNKNOWN_LINUX_GNUEABIHF_LINKER="arm-linux-gnueabihf-gcc" \
    CARGO_TARGET_THUMBV7NEON_UNKNOWN_LINUX_GNUEABIHF_RUSTFLAGS="-C link-args=--sysroot=${SYSROOT_ARMV7HF}" \
    CC_thumbv7neon_unknown_linux_gnueabihf="arm-linux-gnueabihf-gcc" \
    CXX_thumbv7neon_unknown_linux_gnueabihf="arm-linux-gnueabihf-g++"

COPY .devhost/install-system-packages.sh ./
RUN ./install-system-packages.sh \
 && rm install-system-packages.sh

ENV PATH=/usr/local/cargo/bin:$PATH \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup

COPY .devhost/install-rust.sh ./
RUN  ./install-rust.sh \
 && chmod a+w -R $CARGO_HOME $RUSTUP_HOME \
 && rm install-rust.sh

COPY rust-toolchain.toml ./
RUN rustup show \
 && rm rust-toolchain.toml

ENV PATH=/usr/local/venv/bin:$PATH \
    VIRTUAL_ENV=/usr/local/venv

RUN --mount=type=bind,target=/context \
    cd /context/.devhost \
 && ./install-venv.sh $VIRTUAL_ENV \
 && chmod a+w -R $VIRTUAL_ENV

ENV \
    PKG_CONFIG_LIBDIR_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}/usr/lib/pkgconfig:${SYSROOT_AARCH64}/usr/share/pkgconfig" \
    PKG_CONFIG_PATH_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}/usr/lib/pkgconfig:${SYSROOT_AARCH64}/usr/share/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}" \
    PKG_CONFIG_LIBDIR_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}/usr/lib/pkgconfig:${SYSROOT_ARMV7HF}/usr/share/pkgconfig" \
    PKG_CONFIG_PATH_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}/usr/lib/pkgconfig:${SYSROOT_ARMV7HF}/usr/share/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}"

# If neither `CARGO_HOME` nor `HOME` is set when launching a container, then cargo will try to
# download crates to this directory. If launched with the `--user` option then this will fail.
# TODO: Replace the example in the README with something that does not mount any volumes.
RUN mkdir /.cargo \
 && chmod a+w /.cargo/

# `install-venv.sh` install a rust package which creates the cargo registry directory as root.
# TODO: Consider removing the content of `CARGO_HOME` instead of `chmod`ing it;
# it is over 400M already, but that for now having that cached at the start of the dev container may be an advantage.
RUN find $CARGO_HOME $RUSTUP_HOME -type d -exec chmod a+rwx {} +
RUN find $CARGO_HOME $RUSTUP_HOME -type f -exec chmod a+rw {} +

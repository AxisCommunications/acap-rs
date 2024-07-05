ARG REPO=axisecp
ARG SDK=acap-native-sdk
ARG UBUNTU_VERSION=22.04
ARG VERSION=1.15
ARG BASE_IMAGE=debian:bookworm-20240423

FROM ${REPO}/${SDK}:${VERSION}-aarch64-ubuntu${UBUNTU_VERSION} AS sdk-aarch64
FROM ${REPO}/${SDK}:${VERSION}-armv7hf-ubuntu${UBUNTU_VERSION} AS sdk-armv7hf
FROM ${BASE_IMAGE}

COPY --from=sdk-aarch64 /opt/axis/acapsdk/axis-acap-manifest-tools /opt/axis/acapsdk/axis-acap-manifest-tools
COPY --from=sdk-aarch64 /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux /opt/axis/acapsdk/environment-setup-cortexa53-crypto-poky-linux
COPY --from=sdk-armv7hf /opt/axis/acapsdk/environment-setup-cortexa9hf-neon-poky-linux-gnueabi /opt/axis/acapsdk/environment-setup-cortexa9hf-neon-poky-linux-gnueabi
COPY --from=sdk-aarch64 /opt/axis/acapsdk/sysroots/aarch64 /opt/axis/acapsdk/sysroots/aarch64
COPY --from=sdk-armv7hf /opt/axis/acapsdk/sysroots/armv7hf /opt/axis/acapsdk/sysroots/armv7hf
COPY --from=sdk-aarch64 /opt/axis/acapsdk/sysroots/x86_64-pokysdk-linux /opt/axis/acapsdk/sysroots/x86_64-pokysdk-linux

RUN apt-get update \
 && apt-get install -y \
    build-essential \
    clang \
    g++-aarch64-linux-gnu \
    g++-arm-linux-gnueabihf \
    libglib2.0-dev \
    pkg-config \
    iputils-ping \
    python3-jsonschema \
    wget \
 && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

COPY docker/install_rust.sh rust-toolchain.toml ./
RUN ./install_rust.sh \
 && rustup target add \
    aarch64-unknown-linux-gnu \
    thumbv7neon-unknown-linux-gnueabihf \
 && rm install_rust.sh rust-toolchain.toml

ENV \
    SYSROOT_AARCH64=/opt/axis/acapsdk/sysroots/aarch64 \
    SYSROOT_ARMV7HF=/opt/axis/acapsdk/sysroots/armv7hf
# The above makes the below easier to read
ENV \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-args=--sysroot=${SYSROOT_AARCH64}" \
    CC_aarch64_axis_linux_gnu="aarch64-linux-gnu-gcc" \
    CXX_aarch64_axis_linux_gnu="aarch64-linux-gnu-g++" \
    PKG_CONFIG_LIBDIR_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}/usr/lib/pkgconfig:${SYSROOT_AARCH64}/usr/share/pkgconfig" \
    PKG_CONFIG_PATH_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}/usr/lib/pkgconfig:${SYSROOT_AARCH64}/usr/share/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR_aarch64_unknown_linux_gnu="${SYSROOT_AARCH64}" \
    CARGO_TARGET_THUMBV7NEON_UNKNOWN_LINUX_GNUEABIHF_LINKER="arm-linux-gnueabihf-gcc" \
    CARGO_TARGET_THUMBV7NEON_UNKNOWN_LINUX_GNUEABIHF_RUSTFLAGS="-C link-args=--sysroot=${SYSROOT_ARMV7HF}" \
    CC_thumbv7neon_unknown_linux_gnueabihf="arm-linux-gnueabihf-gcc" \
    CXX_thumbv7neon_unknown_linux_gnueabihf="arm-linux-gnueabihf-g++" \
    PKG_CONFIG_LIBDIR_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}/usr/lib/pkgconfig:${SYSROOT_ARMV7HF}/usr/share/pkgconfig" \
    PKG_CONFIG_PATH_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}/usr/lib/pkgconfig:${SYSROOT_ARMV7HF}/usr/share/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR_thumbv7neon_unknown_linux_gnueabihf="${SYSROOT_ARMV7HF}"
ARG ARCH=armv7hf
ARG VERSION=12.0.0
ARG UBUNTU_VERSION=24.04
ARG REPO=axisecp
ARG SDK=acap-native-sdk

FROM ${REPO}/${SDK}:${VERSION}-${ARCH}-ubuntu${UBUNTU_VERSION} AS final
RUN find /opt/axis/acapsdk/sysroots/x86_64-pokysdk-linux/ -name acap-build -delete
COPY target/debug/acap-build /usr/bin/
ENV RUST_BACKTRACE=1 \
    RUST_LOG=debug \
    ACAP_BUILD_RUST=1 \
    SOURCE_DATE_EPOCH=0

#!/usr/bin/env sh
# Extract the ACAP Native SDK from the images it is distributed in to the given `DIRECTORY`.
set -eux

DIRECTORY="${1}"

# Keep version in sync with `Dockerfile`.
docker run axisecp/acap-native-sdk:12.1.0-armv7hf-ubuntu24.04 tar \
  --create \
  --directory /opt/ \
  --file - \
  --mode ugo+rwX \
  axis \
| tar \
  --directory "${DIRECTORY}" \
  --extract \
  --file - \
  --strip-components 1

# Keep version in sync with `Dockerfile`.
docker run axisecp/acap-native-sdk:12.1.0-aarch64-ubuntu24.04 tar \
  --create \
  --directory /opt/ \
  --file - \
  --mode ugo+rwX \
  axis \
| tar \
  --directory "${DIRECTORY}" \
  --extract \
  --file - \
  --strip-components 1

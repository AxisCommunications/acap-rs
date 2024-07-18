#!/usr/bin/env sh
# Extract the ACAP Native SDK from the images it is distributed in to the given `DIRECTORY`.
set -eux

DIRECTORY="${1}"

# Keep version in sync with `Dockerfile`.
docker run axisecp/acap-native-sdk:1.15-armv7hf-ubuntu22.04 tar \
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
docker run axisecp/acap-native-sdk:1.15-aarch64-ubuntu22.04 tar \
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

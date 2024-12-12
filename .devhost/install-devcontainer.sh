#!/usr/bin/env sh
set -eux

# Install `npm` into venv.
# As of writing this is needed because `devcontainer` is a convenient way to test dev containers
# automatically.
curl \
  --location \
  --output /tmp/node-v18.16.1-linux-x64.tar.gz \
  "https://nodejs.org/dist/v18.16.1/node-v18.16.1-linux-x64.tar.gz"

echo "59582f51570d0857de6333620323bdeee5ae36107318f86ce5eca24747cabf5b /tmp/node-v18.16.1-linux-x64.tar.gz" \
| sha256sum -c -

tar -xf "/tmp/node-v18.16.1-linux-x64.tar.gz" --strip-components 1 -C "${VIRTUAL_ENV}"

rm /tmp/node-v18.16.1-linux-x64.tar.gz

# Install `devcontainer` into venv
npm install -g @devcontainers/cli@0.65.0

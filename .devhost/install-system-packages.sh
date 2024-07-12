#!/usr/bin/env sh
set -eux

apt-get update

apt-get install \
  --assume-yes \
  --no-install-recommends \
  build-essential \
  clang \
  curl \
  g++-aarch64-linux-gnu \
  g++-arm-linux-gnueabihf \
  git \
  iputils-ping \
  libglib2.0-dev \
  pkg-config \
  python3-venv

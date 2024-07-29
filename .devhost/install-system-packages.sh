#!/usr/bin/env sh
set -eux

apt-get update

apt-get install \
  --assume-yes \
  --no-install-recommends \
  build-essential \
  clang \
  cmake \
  curl \
  g++-aarch64-linux-gnu \
  g++-arm-linux-gnueabihf \
  git \
  iputils-ping \
  libglib2.0-dev \
  libssl-dev \
  pkg-config \
  python3-venv \
  sshpass

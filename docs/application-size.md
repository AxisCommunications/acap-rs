# Managing application size

- Compilation options
- Avoid large library
- Disable default features

Some techniques affect only the build time, others affect the behavior and may need more careful consideration.

Benchmarks from [application-size.sh](../bin/application-size.sh)

  7.7M	artifacts/baseline/hello_world (This is what is reported in [apps-aarch64.filesize](../apps-aarch64.filesize))
  720K	artifacts/release/hello_world
  420K	artifacts/stable/hello_world
  116K	artifacts/unstable/hello_world
  116K	artifacts/unstable2/hello_world
  112K	artifacts/unstable3/hello_world
  108K	artifacts/unstable4/hello_world

## Compilation options

Consider the optimizations documented in [johnthagen/min-sized-rust](https://github.com/johnthagen/min-sized-rust)

## Avoid large library crates

A good package manager is a double-edged sword.
Library authors face tradeoffs and have to make decisions; libraries may not be optimized for your circumstances.

Crates known to be large include:

- regex
- zbus

Note that large does not necessarily mean wasteful.

## Disable default features

This is especially important if a feature directly or indirectly depends on a particularly large crate.

<!-- TODO: Does deriving serde affect binary size or just compilation time? -->

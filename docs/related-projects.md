# Related projects

This is not the only initiative that facilitates building Rust applications for Axis devices.
Below we compare what some other projects can offer compared to this project.

## cargo-acap

[cargo-acap] is a Rust binary crate that cross-compiles and packages Rust programs for Axis devices.
Installing it and building eap files for virtually every architecture is straight forward:
```sh
cargo install cargo-acap
cargo acap build
```

Advantages:

- Host agnostic:
  - Depends only on Cargo et al. and Docker.
- Requires no ACAP specific boilerplate.
- Supports old products, including ARTPEC-4 and ARTPEC-5.

Disadvantages:

- Unaware of the ACAP manifest:
  - Does not validate the ACAP manifest.
  - Requires information to be duplicated in `Cargo.toml` if the ACAP manifest is to be used.
  - Does not use dynamic user.
- Invokes a container:
  - Cumbersome to use from within another container.
  - Difficult to integrate with IDEs.
- The default container image cannot be used to link the ACAP SDK APIs.
- Assumes custom target triples with the vendor set to `axis`:
  - Restricts how docker images can be built.
  - May cause bugs in crates that use conditional compilation[^1].
- Does not support workspace projects.
- Does not support bundling target-specific files[^2].
- Does not facilitate any interaction with the device including:
  - Compiling and running tests.
  - Installing the built `.eap` file.

## Cross

[cross] is a Rust binary crate that cross-compiles Rust programs for just about any target.

Advantages:

- Host agnostic:
  - Depends only on Cargo et al. and Docker.
- Space efficient:
  - Installs the bulky Rust toolchains on host and mounts them into the container.
- Supports emulation.

Disadvantages:

- Unaware of ACAP.
- Invokes a container:
  - Cumbersome to use from within another container.
  - Difficult to integrate with IDEs.
  - Difficult to wrap with other tools[^3].

[cargo-acap]: https://github.com/trunnion/cargo-acap
[cross]: https://github.com/cross-rs/cross
[^1]: https://github.com/trunnion/cargo-acap/commit/6748c52ef1c13a6a12cc327a65a333c012c5725b
[^2]: This has been discussed but not implemented https://github.com/trunnion/cargo-acap/pull/5
[^3]: This project initially used cross and struggled with TTY-stuff.

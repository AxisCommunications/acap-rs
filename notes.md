Using SOURCE_DATE_EPOCH seems to make builds reproducible across time, but not machines.

I have so far failed to use `--remap-path-prefix`, I suspect that the env vars override my config but I have not bothered to confirm.

Using a nightly toolchain would enable the use of the unstable [trim-paths](https://doc.rust-lang.org/cargo/reference/unstable.html#profile-trim-paths-option) feature.
For this to work in a convenient way, `cargo-acap-sdk` or similar would have to propagate the toolchain argument to `cargo`.
This can be easily, but somewhat clumsily, implemented in clap with a custom value parser.

TODO: Propagate toolchain to cargo?
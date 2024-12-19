# Device Tests

```bash
cargo test --tests -p larod --features device-tests --target aarch64-unknown-linux-gnu -- --nocapture 
```

# Compilation Tests
Because this library is a wrapper around a C library, the `unsafe` keyword is used extensively. One of the nice things about Rust is the compile time guarantees around lifetimes. In developing this library, we intend to produce an API that upholds Rusts compile time guarantees and prevents compilation when the developer is trying to do something unsafe. However, testing this to verify we're doing what we intend to do is tricky, because we can't really run a test if we can't compile it (by design).

So, the trybuild crate is used along with a special test organization to intentially write test Rust programs that don't compile. The trybuild crate can then test and verify that the code does indeed not compile, and with the expected error. The trybuild crate will look for Rust files and compile them and match the comipler error to an expected error. Since it will look for a local file, these tests must be run on the development host, and cannot be shipped to a remote camera along with other device tests. So, `AXIS_DEVICE_IP` must be unset, the target architecture must be the native host architecture, and we must request that normally ignored tests be run.

An example invocation is:
```bash
unset AXIS_DEVICE_IP
cargo test --tests -p larod compiler_tests::lifetimes -- --ignored --nocapture
```
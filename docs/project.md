# Priorities

This section provides guidance on how to determine if a contribution should be accepted or not.

## ACAP Native API bindings

The quality of a library has many facets:

- Ergonomics:
  - How straightforward to use are the bindings
  - How readable are applications using the bindings
  - How fast do the bindings compile
  - How much can be achieved without `unsafe`
  - etc
- Flexibility: What portion of the programs that one may want to express with the underlying C API can be expressed with the Rust bindings.
- Performance: How much storage, memory and CPU does an application built using the bindings use.
- Robustness: Can non-breaking changes to the underlying C API be made without breaking the Rust bindings and could such.
- Soundness: Safe Rust using the library cannot cause undefined behavior [^1].
- Stability: Can changes to the underlying C API be propagated to the Rust bindings without making breaking changes.

This project prioritizes these facets as follows:

1. Soundness
2. Robustness
3. Flexibility and stability
4. Ergonomics and performance

Consequences of this include:

- A contribution that improves soundness should always be accepted, even if it deteriorates performance.
- A contribution that deteriorates soundness should never be accepted, even if it improves performance.
- A contribution that improves flexibility should be accepted, even if it deteriorates performance.

The above should be considered a default, not a hard rule.

# Versioning

According to [Semantic Versioning](https://semver.org/spec/v2.0.0.html#spec-item-4) 2.0.0:

> Major version zero (0.y.z) is for initial development. Anything MAY change at any time. The public API SHOULD NOT be considered stable.

Since everything in this project is in "initial development", adding some nuance may be helpful.
In general, more leading zeros should be interpreted as less likely to work and more likely to change.
More specific remarks include:

- `0.0.0`: The crate is not (yet) useful outside of this project.
  As such, it is not published.
  It may nonetheless serve as inspiration or a starting point for other projects.
- `0.0.z` where `z>0`: The crate may be useful outside of this project, but users should expect to read the source code in order to use it.
- `0.y.z` where `y>0`: The crate may be useful outside of this project.

The name of a package is subject to change until it has been published, regardless of version.

[^1]: https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html

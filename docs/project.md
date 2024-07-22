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

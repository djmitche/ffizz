# Releasing

* Update version in `*/Cargo.toml`, including inter-crate version references.
* Commit `-m vX.Y.Z`
* Tag `-m vX.Y.Z`
* Cargo publish, in order:
  * `cargo publish -p ffizz-passby`
  * `cargo publish -p ffizz-macros`
  * `cargo publish -p ffizz-headers`
  * `cargo publish -p ffizz-string`

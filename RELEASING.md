# Releasing

* Update version in `*/Cargo.toml`, including inter-crate version references.
* Commit `git commit -am vX.Y.Z`
* Tag `git tag vX.Y.Z`
* Cargo publish, in order:
  * `cargo publish -p ffizz-passby`
  * `cargo publish -p ffizz-macros`
  * `cargo publish -p ffizz-headers`
  * `cargo publish -p ffizz-string`
* Git push

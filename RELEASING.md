# Releasing

* Update version in `*/Cargo.toml`, including inter-crate version references.
* Commit `git commit -am vX.Y.Z`
* Tag `git tag vX.Y.Z`
* `git push` -- automation will do the rest

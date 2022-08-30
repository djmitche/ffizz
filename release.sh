#! /bin/bash

set -e

# ordered by dependencies
cargo publish -p ffizz-passby
cargo publish -p ffizz-macros
cargo publish -p ffizz-header
cargo publish -p ffizz-string

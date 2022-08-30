#! /bin/bash

set -e

# ordered by dependencies, with sleep's in between to allow crates.io's DB to
# catch up
cargo publish -p ffizz-passby
sleep 10
cargo publish -p ffizz-macros
sleep 10
cargo publish -p ffizz-header
sleep 10
cargo publish -p ffizz-string

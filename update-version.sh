#! /bin/bash

VERSION=$1

if [ -z "$VERSION" ] || [[ "$VERSION" = v* ]]; then
    echo "USAGE: $0 <version> (without leading v)"
    exit 1
fi

for f in */Cargo.toml; do
    sed -i '/^\(version\|ffizz-.* = {.*version = \)/s/version = "[0-9.]*"/version = "'${VERSION}'"/g' $f
done

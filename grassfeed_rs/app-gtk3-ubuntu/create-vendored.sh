#!/bin/bash
test -f Cargo.lock && rm -rf Cargo.lock
test -f vendor && rm -rf vendor
cargo vendor
rm vendor/windows* -rf
rm vendor/winapi* -rf
mv vendor sources-vendored
tar c sources-vendored |gzip >src_vendored.tar.gz
test -d target || mkdir target
rm -rf target/sources-vendored
mv -v sources-vendored target/sources-vendored

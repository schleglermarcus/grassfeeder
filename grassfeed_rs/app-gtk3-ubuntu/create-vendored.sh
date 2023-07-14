#!/bin/bash
test -f vendor && rm -rf vendor
cargo vendor
mv vendor sources-vendored
tar c sources-vendored |gzip >src_vendored.tar.gz
mv -v sources-vendored target/
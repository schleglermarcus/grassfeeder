#!/bin/bash
#
# Various libraries (dir_next, chrono, ... ) attract windows libraries, so i cannot delete them here. 
# This increases the source tar ball significantly.
#
test -f Cargo.lock && rm -rf Cargo.lock
test -f vendor && rm -rf vendor
cargo vendor
find vendor/windows* -name src -type d |xargs rm -rfv
find vendor/windows* -name lib -type d |xargs rm -rfv
find vendor/winapi* -name lib -type d |xargs rm -rfv
find vendor/winapi* -name src -type d |xargs rm -rfv
find vendor/winnow -name src -type d |xargs rm -rfv
find vendor/winnow -name examples -type d |xargs rm -rfv

mv vendor sources-vendored
tar c sources-vendored |gzip >src_vendored.tar.gz
test -d target || mkdir target
rm -rf target/sources-vendored
mv -v sources-vendored target/sources-vendored

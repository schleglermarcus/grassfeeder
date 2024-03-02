#!/bin/bash
#
# Various libraries (dir_next, chrono, ... ) attract windows libraries, so i cannot delete them here. 
# This increases the source tar ball significantly.
#
test -f Cargo.lock && rm -rf Cargo.lock
test -f vendor && rm -rf vendor
cargo vendor

find vendor/windows/src      -name Windows -type d |xargs rm -rf
find vendor/windows-sys/src  -name Windows -type d |xargs rm -rf
find vendor/windows-sys*     -name Windows -type d |xargs rm -rf
find vendor/windows* -name lib -type d |xargs rm -rf
find vendor/winapi* -name lib -type d |xargs rm -rf

find vendor/image -name tests -type d |xargs rm -rfv


# That file sqlite3.c is big, and still needed to build
# find vendor/libsqlite3-sys/sqlcipher -name sqlite3.c  -type f |xargs rm -rfv


mv vendor sources-vendored
tar c sources-vendored |gzip >src_vendored.tar.gz
test -d target || mkdir target
rm -rf target/sources-vendored
mv -v sources-vendored target/sources-vendored

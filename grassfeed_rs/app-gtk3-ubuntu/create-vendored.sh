#!/bin/bash
#
# Various libraries (dir_next, chrono, ... ) attract windows libraries, so i cannot delete them here.
# This increases the source tar ball significantly.
#
#   image/tests and image/examples is used by the image lib to build :-(
#   sqlite3.c is big, and still needed to build
#
test -f Cargo.lock && rm -rf Cargo.lock
test -f vendor && rm -rf vendor
cargo vendor

##        ‘vendor/windows/src’: Datei oder Verzeichnis nicht gefunden
##   find vendor/windows/src      -name Windows -type d |xargs rm -rf

find vendor/windows-sys/src  -name Windows -type d |xargs rm -rf
find vendor/windows-sys*     -name Windows -type d |xargs rm -rf
find vendor/windows* -name lib -type d |xargs rm -rf
find vendor/winapi* -name lib -type d |xargs rm -rf


DEST="sources-vendored"
test -d target || mkdir target
find target/sources_* |xargs rm -rf
test -d $DEST || rm -rf $DEST
mkdir $DEST

mv -v vendor/soup3 $DEST/
mv -v vendor/libsqlite3* $DEST/
mv -v vendor/web-sys $DEST/
mv -v vendor/image* $DEST/
mv -v vendor/ring  $DEST/
mv -v vendor/libwebp-sys2  $DEST/

tar c $DEST |gzip >src_vendored_1.tar.gz

test -d target/sources_vendored_1 || rm -rf target/sources_vendored_1
mv -v sources-vendored target/sources_vendored_1

mv vendor sources-vendored
tar c sources-vendored |gzip >src_vendored_2.tar.gz
test -d target/sources_vendored_2 || rm -rf target/sources_vendored_2
mv -v sources-vendored target/sources_vendored_2




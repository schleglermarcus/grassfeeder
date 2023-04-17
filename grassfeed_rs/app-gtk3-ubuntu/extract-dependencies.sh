#!/bin/bash
cargo deb
test -d target || mkdir target
DEBFILE=`find ../target/debian/grassfeeder*.deb`
DEBFILE="../$DEBFILE"

(cd target ;    ar -x  "$DEBFILE"   )
(cd target ;  cat control.tar.xz | unxz -d |tar x )
cp -v target/control assets/deb-control.txt


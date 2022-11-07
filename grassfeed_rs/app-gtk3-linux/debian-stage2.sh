#!/bin/bash

DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
rm -rf target
mkdir target

# the output file is used by the docker file
(cd ../../ ; tar c --exclude=grassfeed_rs/target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/gf.tar.gz )

docker build -t grassfeeder:debian-stage2 -f  debian-stage2.docker .


echo "== debian-stage2.2h copying   gf.deb"

docker cp $(docker create --name tc grassfeeder:debian-stage2):/usr/src/grassfeed_rs/app-gtk3-linux/gf.AppImage target/ ; docker rm tc
mv target/gf.AppImage  target/grassfeeder-${VERSION}-debian11.AppImage
ls -lh target/

docker cp $(docker create --name tc grassfeeder:debian-stage2):/usr/src/grassfeed_rs/target/gf.deb target/ ; docker rm tc
mv target/gf.deb  target/grassfeeder-${VERSION}-debian11.deb


docker cp $(docker create --name tc grassfeeder:debian-stage2):/usr/src/grassfeed_rs/target/gf.rpm target/ ; docker rm tc
mv target/gf.rpm  target/grassfeeder-${VERSION}-debian11.rpm

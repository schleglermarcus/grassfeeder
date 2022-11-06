#!/bin/bash

DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
rm -rf target
mkdir target

# the output file is used by the docker file
(cd ../../ ; tar c --exclude=grassfeed_rs/target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/gf.tar.gz )

docker build -t grassfeeder:debian-stage1 -f  debian2.docker .


docker cp $(docker create --name tc grassfeeder:debian-stage2):/usr/src/grassfeed_rs/target/gf.deb target/ ; docker rm tc
mv target/gf.deb  target/grassfeeder-${VERSION}_x86_64.deb

#docker cp $(docker create --name tc grassfeeder:stage2):/usr/src/grassfeed_rs/app-gtk3-linux/gf.AppImage target/ ; docker rm tc
#mv target/gf.AppImage  target/grassfeeder-${VERSION}-x86_64.AppImage

docker cp $(docker create --name tc grassfeeder:debian-stage2):/usr/src/grassfeed_rs/target/gf.rpm target/ ; docker rm tc
mv target/gf.rpm  target/grassfeeder-${VERSION}_x86_64.rpm

#!/bin/bash
test -d target || mkdir target
DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
echo "VERSION=$VERSION	"
# the output file is used by the docker file            >$DIR/target/grassfeeder-${VERSION}.tar.gz
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/gf.tar.gz )

ls -l target/

docker build  --build-arg GFTAR=grassfeeder-${VERSION}.tar.gz -t grassfeeder:ubuntu-stage2 -f  ubuntu-stage2.docker .



# docker cp $(docker create --name tc grassfeeder:debian-stage4):/usr/src/grassfeed_rs/target/gf.deb target/ ; docker rm tc
# mv target/gf.deb  target/grassfeeder-${VERSION}-debian11.deb


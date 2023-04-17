#!/bin/bash
test -d target || mkdir target
DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
echo "VERSION=$VERSION	DEBFILE=$DEBFILE"
# the output file is used by the docker file
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/grassfeeder-${VERSION}.tar.gz )


docker build -t grassfeeder:debian-stage4 -f  stage4.docker .



# docker cp $(docker create --name tc grassfeeder:debian-stage4):/usr/src/grassfeed_rs/target/gf.deb target/ ; docker rm tc
# mv target/gf.deb  target/grassfeeder-${VERSION}-debian11.deb


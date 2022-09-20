#!/bin/bash

DIR=`pwd` 
test -d target || mkdir target

# the output file is used by the docker file
(cd ../../ ; tar c --exclude=grassfeed_rs/target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/gf.tar.gz )

docker build -t grassfeeder:stage2 -f  stage2.docker .

docker cp $(docker create --name tc grassfeeder:stage2):/usr/src/gf/grassfeed_rs/target/debian/grassfeeder*.deb target/ ; docker rm tc


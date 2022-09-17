#!/bin/bash

mkdir target
docker build -t grassfeeder:stage2 -f  stage2.docker .

docker cp $(docker create --name tc grassfeeder:stage2):/usr/src/gf/grassfeed_rs/target/debian/grassfeeder_0.0.4_amd64.deb target/ ; docker rm tc


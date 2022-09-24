#!/bin/bash
rm -rf target
mkdir target
(tar c --exclude=target --exclude=Cargo.lock   .  |gzip --fast  >target/cr_src.tar.gz )

docker build -t grassfeeder:fedora1 -f fedora-stage2.docker .

docker cp $(docker create --name tc grassfeeder:fedora2):/usr/src/out_package.zip target/ ; docker rm tc
ls -l target/




#!/bin/bash
# https://linuxconfig.org/easy-way-to-create-a-debian-package-and-local-package-repository 
#



export REPREPRO_BASE_DIR=`pwd`

DEBS=`find . -name "*.deb" |xargs`

reprepro includedeb grassfeeder $DEBS

# dpkg-scanpackages . | gzip -c9  > Packages.gz
# dpkg-scanpackages -m pool | gzip  > dists/grassfeeder/Packages.gz


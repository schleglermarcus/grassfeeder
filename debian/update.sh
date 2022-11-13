#!/bin/bash
# https://linuxconfig.org/easy-way-to-create-a-debian-package-and-local-package-repository 
#

export REPREPRO_BASE_DIR=`pwd`

DEBS=`find . -name "*.deb" |xargs`

test -d dists && mv dists dists.old
test -d db && mv db db.old

reprepro --delete clearvanished 
reprepro includedeb grassfeeder $DEBS

# dpkg-scanpackages . | gzip -c9  > Packages.gz
# dpkg-scanpackages -m pool | gzip  > dists/grassfeeder/Packages.gz
#  DEBS    





# apt-ftparchive --md5 --sha256 release .  > Release 
# gpg --digest-algo SHA256 --armor --output Release.gpg --detach-sign Release
# gpg --digest-algo SHA256 --clearsign --output InRelease Release

# https://unix.stackexchange.com/questions/387053/debian-9-apt-and-gpg-error-inrelease-the-following-signatures-were-inva
# Adjust the personal-digest-preferences and personal-cipher-preferences 
# in $HOME/.gnupg/gpg.conf to eliminate SHA-1 from one's GPG preferences. 
# This prevents the problem coming back with new keys.




#!/bin/bash
#  Update the repository tree, with the new *.deb

export REPREPRO_BASE_DIR=`pwd`
test -d dists && mv dists ../grassfeed_rs/target/dists.old
test -d db && mv db ../grassfeed_rs/target/db.old

DEBS=`find . -name "*.deb" |xargs`
reprepro --delete clearvanished 
reprepro includedeb grassfeeder $DEBS



# https://unix.stackexchange.com/questions/387053/debian-9-apt-and-gpg-error-inrelease-the-following-signatures-were-inva
# Adjust the personal-digest-preferences and personal-cipher-preferences 
# in $HOME/.gnupg/gpg.conf to eliminate SHA-1 from one's GPG preferences. 
# This prevents the problem coming back with new keys.



# apt-ftparchive --md5 --sha256 release .  > Release 
# gpg --digest-algo SHA256 --armor --output Release.gpg --detach-sign Release
# gpg --digest-algo SHA256 --clearsign --output InRelease Release

#  W: Das Laden der konfigurierten Datei »binary/binary-all/Packages« wird übersprungen, 
# da das Depot »https://raw.githubusercontent.com/schleglermarcus/grassfeeder/main/debian grassfeeder InRelease« 
# die Architektur »all« nicht unterstützt.

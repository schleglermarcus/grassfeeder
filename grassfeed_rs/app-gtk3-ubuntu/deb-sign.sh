#!/bin/bash
# Preparing the source code to be uploaded on the personal package archive
# Needs installed:   rustc cargo devscripts
#
#  Remark: debcargo  ia a inner-debian tool for converting libraries
#
#	dput requirements:
#   {'allowed': ['release'], 'known': ['release', 'proposed', 'updates', 'backports', 'security']}
#


PKGNAME="grassfeeder-gtk3"
T_MAINT="Marcus <schlegler_marcus@posteo.de>"

DIR=`pwd`
# VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
VERSION=`cat ../resources/version.txt`

test -d target || mkdir target
test -d ../target || mkdir ../target
rm -rf target/deb-sign/*

WORK="$DIR/target/deb-sign"
echo "VERSION=$VERSION	    WORKDIR=$WORK"


## no-docker
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >grassfeed_rs/target/${PKGNAME}-${VERSION}.tar.gz )
test -d $WORK || mkdir $WORK
mkdir $WORK/${PKGNAME}-${VERSION}


UNPACK="../target/${PKGNAME}-${VERSION}.tar.gz"
(cd $WORK/${PKGNAME}-$VERSION ; cat  ../../../$UNPACK |gzip -d |tar x )

mkdir $WORK/${PKGNAME}-$VERSION/debian
cp -v assets/changelog.txt $WORK/${PKGNAME}-$VERSION/debian/changelog

mkdir $WORK/${PKGNAME}-$VERSION/debian/source
echo "1.0" >$WORK/${PKGNAME}-$VERSION/debian/source/format

CT=$WORK/${PKGNAME}-$VERSION/debian/control
echo "Source: $PKGNAME" >$CT
echo "Section: web" >>$CT
echo "Priority: optional" >>$CT
echo "Maintainer: $T_MAINT" >>$CT
echo "Build-Depends: rustc, cargo, devscripts"  >>$CT
echo "" >>$CT
echo "Package: $PKGNAME"  >>$CT
cat assets/deb-control.txt |egrep  "Architecture:"  |head -n1  >>$CT
cat assets/deb-control.txt |egrep  "Depends:"  |head -n1  >>$CT

R="debian/rules"
(cd $WORK/${PKGNAME}-$VERSION ;   echo "#!/usr/bin/make -f" >$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "clean:" >>$R )

(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./unpack-vendored.sh ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; cargo clean ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "build: " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./deb-create.sh ) " >>$R )
# (cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; cargo --offline build --release  ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )

# (cd $WORK/grassfeeder-$VERSION ;  FAKEROOTKEY=1 LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libfakeroot/libfakeroot-0.so   dpkg-buildpackage -us -uc -ui -S -rfakeroot   )
(cd $WORK/${PKGNAME}-$VERSION ;   debuild  -rfakeroot -S  )


#echo "----"

( cd $WORK ; echo "dput ppa:schleglermarcus/ppa  `ls -1 grassfeeder*source.changes`" )


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
T_MAINT="Marcus der Schlegler <schlegler_marcus@posteo.de>"
BUILD_DEPENDS="rustc, cargo, devscripts, pkg-config, librust-glib-dev, librust-glib-sys-dev, librust-gobject-sys-dev, libatk1.0-dev, libwebkit2gtk-4.0-dev, libsoup2.4-dev "

DIR=`pwd`
VERSION=`cat ../resources/version.txt`
SECTION="web"
PRIORITY="optional"
ARCHITECTURE="amd64"

(cd ../resources; cargo test )

rm -rf ../testing/target
rm -rf target
mkdir target

WORK="$DIR/target/deb-sign"
echo "VERSION=$VERSION	    WORKDIR=$WORK"
test -d ../target || mkdir ../target
EXCL="--exclude=grassfeed_rs/target --exclude=grassfeed_rs/app-gtk3-ubuntu/target --exclude=grassfeed_rs/Cargo.lock "
(cd ../../ ; tar c $EXCL  grassfeed_rs |gzip --fast  >grassfeed_rs/target/${PKGNAME}-${VERSION}.tar.gz )
test -d $WORK || mkdir $WORK
mkdir $WORK/${PKGNAME}-${VERSION}
UNPACK="../target/${PKGNAME}-${VERSION}.tar.gz"
(cd $WORK/${PKGNAME}-$VERSION ;  cat  ../../../$UNPACK |gzip -d |tar x )
mkdir $WORK/${PKGNAME}-$VERSION/debian
cp -v assets/changelog.txt $WORK/${PKGNAME}-$VERSION/debian/changelog

mkdir $WORK/${PKGNAME}-$VERSION/debian/source
echo "1.0" >$WORK/${PKGNAME}-$VERSION/debian/source/format

CT=$WORK/${PKGNAME}-$VERSION/debian/control
echo "Source: $PKGNAME" >$CT
echo "Section: $SECTION" >>$CT
echo "Priority: $PRIORITY" >>$CT
echo "Maintainer: $T_MAINT" >>$CT
echo "Build-Depends: $BUILD_DEPENDS "  >>$CT
echo "" >>$CT
echo "Package: $PKGNAME"  >>$CT
echo "Architecture: $ARCHITECTURE"  >>$CT
cat assets/deb-control.txt |egrep  "Depends:"  |head -n1  >>$CT
cp -vR $CT debian/

R="debian/rules"
(cd $WORK/${PKGNAME}-$VERSION ;   echo "#!/usr/bin/make -f" >$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "clean:" >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; cargo clean ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "build: " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./unpack-vendored.sh ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./deb-create.sh ) " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "binary: " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	cp -v grassfeed_rs/app-gtk3-ubuntu/target/grassfeeder*.deb ../${PKGNAME}_${VERSION}_${ARCHITECTURE}.deb  " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	find . -name \"files\"  " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	if test  -f debian/files ; then  mv -v debian/files debian/files.1 ; fi   " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	if ! test -d debian       ; then mkdir debian ; fi " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	dpkg-distaddfile ${PKGNAME}_${VERSION}_${ARCHITECTURE}.deb $SECTION $PRIORITY" >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "	ls -lR debian/  " >>$R )
(cd $WORK/${PKGNAME}-$VERSION ;   echo "">>$R )

(cd $WORK/${PKGNAME}-$VERSION ;   debuild  -rfakeroot -S  )

( cd $WORK ; echo "# (cd target/deb-sign/ ; dput ppa:schleglermarcus/grassfeeder  `ls -1 grassfeeder*source.changes |tail -n1` )" )

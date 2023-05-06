#!/bin/bash
# Preparing the source code to be uploaded on the personal package archive
# Needs installed:   rustc cargo devscripts
#
#  Remark: debcargo  ia a inner-debian tool for converting libraries
#
#	dput requirements:
#   {'allowed': ['release'], 'known': ['release', 'proposed', 'updates', 'backports', 'security']}
#
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

FOLDER_SIGN="$DIR/target/deb-sign"
echo "VERSION=$VERSION	    FOLDER_SIGN=$FOLDER_SIGN"
test -d ../target || mkdir ../target
EXCL="--exclude=grassfeed_rs/target --exclude=grassfeed_rs/app-gtk3-ubuntu/target --exclude=grassfeed_rs/Cargo.lock "
(cd ../../ ; tar c $EXCL  grassfeed_rs |gzip --fast  >grassfeed_rs/target/${PKGNAME}-${VERSION}.tar.gz )
test -d $FOLDER_SIGN || mkdir $FOLDER_SIGN

WORKDIR=${FOLDER_SIGN}/${PKGNAME}-${VERSION}
mkdir $WORKDIR
UNPACK="../target/${PKGNAME}-${VERSION}.tar.gz"
(cd $WORKDIR ;  cat  ../../../$UNPACK |gzip -d |tar x )
mkdir $WORKDIR/debian
cp -v assets/changelog.txt $WORKDIR/debian/changelog

mkdir $WORKDIR/debian/source
echo "1.0" >$WORKDIR/debian/source/format

CT=${WORKDIR}/debian/control
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
(cd $WORKDIR ;   echo "#!/usr/bin/make -f" >$R )
(cd $WORKDIR ;   echo "">>$R )
(cd $WORKDIR ;   echo "clean:" >>$R )
(cd $WORKDIR ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; cargo clean ) " >>$R )
(cd $WORKDIR ;   echo "">>$R )
(cd $WORKDIR ;   echo "build: " >>$R )
(cd $WORKDIR ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./unpack-vendored.sh ) " >>$R )
(cd $WORKDIR ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./deb-create.sh ) " >>$R )
(cd $WORKDIR ;   echo "">>$R )
(cd $WORKDIR ;   echo "binary: " >>$R )
(cd $WORKDIR ;   echo "	cp -v grassfeed_rs/app-gtk3-ubuntu/target/grassfeeder*.deb ../${PKGNAME}_${VERSION}_${ARCHITECTURE}.deb  " >>$R )
(cd $WORKDIR ;   echo "	find . -name \"files\"  " >>$R )
(cd $WORKDIR ;   echo "	if test  -f debian/files ; then  mv -v debian/files debian/files.1 ; fi   " >>$R )
(cd $WORKDIR ;   echo "	if ! test -d debian       ; then mkdir debian ; fi " >>$R )
(cd $WORKDIR ;   echo "	dpkg-distaddfile ${PKGNAME}_${VERSION}_${ARCHITECTURE}.deb $SECTION $PRIORITY" >>$R )
(cd $WORKDIR ;   echo "	ls -lR debian/  " >>$R )
(cd $WORKDIR ;   echo "">>$R )
(cd $WORKDIR ;   debuild  -rfakeroot -S  )

CHANGES=`( cd $FOLDER_SIGN ;  ls -1 grassfeeder*source.changes |tail -n1 ) `
echo "# (cd target/deb-sign/ ; dput ppa:schleglermarcus/grassfeeder  $CHANGES )"

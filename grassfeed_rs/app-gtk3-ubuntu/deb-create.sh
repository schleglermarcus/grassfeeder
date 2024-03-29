#!/bin/bash
PKGNAME="grassfeeder-gtk3"
DIR=`pwd`
T_MAINT="Marcus <schlegler_marcus@posteo.de>"
T_LICENSE="LGPL-3"
VERSION=`cat ../resources/version.txt`

WORK="$DIR/target/$PKGNAME-$VERSION"
test -d $DIR/target || mkdir $DIR/target
echo "VERSION=$VERSION	    WORKDIR=$WORK"

cargo --offline test --release
cargo --offline build --release

rm -rf $WORK  ;
mkdir $WORK
test -d $WORK/DEBIAN || mkdir $WORK/DEBIAN

mkdir $WORK/usr
mkdir $WORK/usr/bin
cp -v  ../target/release/grassfeeder  $WORK/usr/bin/
mkdir $WORK/usr/share
mkdir $WORK/usr/share/doc
mkdir $WORK/usr/share/doc/$PKGNAME
cat assets/changelog.txt |gzip >$WORK/usr/share/doc/$PKGNAME/changelog.gz
mkdir $WORK/usr/share/applications
cp assets/grassfeeder.desktop  $WORK/usr/share/applications/
mkdir $WORK/usr/share/pixmaps
mkdir $WORK/usr/share/pixmaps/grassfeeder
cp assets/grassfeeder.xpm  $WORK/usr/share/pixmaps/grassfeeder/

INST_SIZE=`(cd  $WORK ;  du -ks usr|cut -f 1)`

CT=$WORK/DEBIAN/control
echo "Package: $PKGNAME" >$CT
echo "Version: $VERSION" >>$CT
echo "Architecture: amd64" >>$CT
echo "Section: web" >>$CT
echo "Priority: optional" >>$CT
echo "Maintainer: $T_MAINT" >>$CT
echo "Installed-Size:$INST_SIZE"  >>$CT
cat assets/deb-control.txt |egrep  "Depends:"  |head -n1  >>$CT
cat assets/deb-control.txt |egrep  "Description:" -A3 |head -n4  >>$CT

CP=$WORK/usr/share/doc/$PKGNAME/copyright
echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/" >$CP
echo "Upstream-Name: $PKGNAME" >>$CP
echo "Copyright: 2023 $T_MAINT" >>$CP
echo "License: $T_LICENSE" >>$CP

FILES=`(cd $WORK ; find . -type f |grep -v DEBIAN |sort) |xargs`
echo "md5sum-FILES: $FILES"
for F in $FILES
do
    (cd $WORK ; echo $F >> DEBIAN/files;  md5sum $F >>DEBIAN/md5sums )
done

(cd target ; dpkg-deb --root-owner-group --build $PKGNAME-$VERSION )

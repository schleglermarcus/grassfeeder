#!/bin/bash
DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
test -d target || mkdir target

echo "VERSION=$VERSION" 
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/grassfeeder-${VERSION}.tar.gz )

#  using  $DIR/target/gf.tar.gz    from stage2
WORK="$DIR/target/deb-sign"
mkdir $WORK
(cd $WORK ;    ar -x ../gf.deb )
(cd $WORK ;  cat control.tar.xz | unxz -d |tar x )
mkdir $WORK/grassfeeder-$VERSION
(cd $WORK/grassfeeder-$VERSION ; cat  $DIR/target/grassfeeder-${VERSION}.tar.gz |gzip -d |tar x )

mkdir $WORK/grassfeeder-$VERSION/debian
cp -v assets/changelog.txt $WORK/grassfeeder-$VERSION/debian/changelog

mkdir $WORK/grassfeeder-$VERSION/debian/source
echo "1.0" >$WORK/grassfeeder-$VERSION/debian/source/format 

CT=$WORK/grassfeeder-$VERSION/debian/control
echo "Source: grassfeeder" >$CT
echo "Section: web" >>$CT
echo "Priority: optional" >>$CT
echo "Maintainer: Marcus <schlegler_marcus@posteo.de>" >>$CT
echo "" >>$CT
cat $WORK/control |egrep -v "Version:"  >>$CT


(cd $WORK/grassfeeder-$VERSION ;   touch debian/rules )
(cd $WORK/grassfeeder-$VERSION ;   debuild -S )
(cd $WORK/grassfeeder-$VERSION ;   pwd )




## dput ppa:schleglermarcus/grassfeeder-ppa  grassfeeder_0.1.7_source.changes 
#Uploading grassfeeder using ftp to ppa (host: ppa.launchpad.net; directory: ~schleglermarcus/grassfeeder-ppa)
#running supported-distribution: check whether the target distribution is currently supported (using distro-info)
#{'allowed': ['release'], 'known': ['release', 'proposed', 'updates', 'backports', 'security']}
#Unknown release unstable


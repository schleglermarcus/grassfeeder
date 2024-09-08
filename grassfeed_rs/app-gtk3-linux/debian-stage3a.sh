#!/bin/bash
DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
test -d target || mkdir target

DEBFILE=`find ../target/debian -iname grassfeeder_*.deb |head -n1`


echo "VERSION=$VERSION	DEBFILE=$DEBFILE"
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/grassfeeder-${VERSION}.tar.gz )


#  using  $DIR/target/gf.tar.gz    from stage2
WORK="$DIR/target/deb-sign"
mkdir $WORK
(cd $WORK ;    ar -x  "../../$DEBFILE"  )		#	../gf.deb
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
cat $WORK/control |grep  "Version:" |head -n1  >>$CT
cat $WORK/control |egrep  "Package:" |head -n1  >>$CT
cat $WORK/control |egrep  "Architecture:"  |head -n1  >>$CT
cat $WORK/control |egrep  "Depends:"  |head -n1  >>$CT

cat $CT


R="debian/rules"
(cd $WORK/grassfeeder-$VERSION ;   echo "#!/usr/bin/make -f" >$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "clean:" >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-linux/ ; cargo clean ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "build: " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-linux/ ; cargo deb ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )

(cd $WORK/grassfeeder-$VERSION ;   debuild  -rfakeroot -S  )

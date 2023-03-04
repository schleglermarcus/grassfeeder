#!/bin/bash
DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
test -d target || mkdir target

DEBFILE=`find ../target/debian -iname grassfeeder_*.deb |head -n1`


echo "VERSION=$VERSION	DEBFILE=$DEBFILE"
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/grassfeeder-${VERSION}.tar.gz )

# ../target/debian/grassfeeder_0.1.8~B1_amd64.deb
# DEBFILE="../target/grassfeeder-$VERSION-"


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
#(cd $WORK/grassfeeder-$VERSION ;   echo "	apt update " >>$R )
#(cd $WORK/grassfeeder-$VERSION ;   echo "	apt install -y wget git pkgconf librust-glib-sys-dev libatk1.0-dev librust-gdk-sys-dev libsoup2.4-dev libjavascriptcoregtk-4.0-dev libwebkit2gtk-4.0-dev " >>$R )
#(cd $WORK/grassfeeder-$VERSION ;   echo "	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -sSf | sh -s -- -y	 " >>$R )
#(cd $WORK/grassfeeder-$VERSION ;   echo "	export PATH=/root/.cargo/bin:$PATH " >>$R )
# (cd $WORK/grassfeeder-$VERSION ;   echo "	cargo install cargo-deb " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-linux/ ; cargo clean ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "build: " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-linux/ ; cargo deb ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )

# (cd $WORK/grassfeeder-$VERSION ;  FAKEROOTKEY=1 LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libfakeroot/libfakeroot-0.so   dpkg-buildpackage -us -uc -ui -S -rfakeroot   )
(cd $WORK/grassfeeder-$VERSION ;   debuild  -rfakeroot -S  )





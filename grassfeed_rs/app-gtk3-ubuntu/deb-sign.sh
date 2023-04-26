#!/bin/bash
test -d target || mkdir target

DIR=`pwd`
VERSION=`cat Cargo.toml  |grep "^version"  |sed -e "s/.*= \"//" -e "s/\"//"`
echo "VERSION=$VERSION	DEBFILE=$DEBFILE"

## no-docker
(cd ../../ ; tar c --exclude=target --exclude=grassfeed_rs/Cargo.lock   grassfeed_rs  |gzip --fast  >$DIR/target/grassfeeder-${VERSION}.tar.gz )

#cargo deb
#DEBFILE=`find ../target/debian -iname grassfeeder_*.deb |head -n1`
# DEBFILENAME=`basename $DEBFILE`

WORK="$DIR/target/deb-sign"
test -d $WORK || mkdir $WORK
mkdir $WORK/grassfeeder-${VERSION}

# cp -v $DEBFILE  $WORK/
# (cd $WORK ;    ar -x  "$DEBFILENAME"  )
# (cd $WORK ;  cat control.tar.xz | unxz -d |tar x )
# mkdir "$WORK/grassfeeder-$VERSION"

(cd $WORK/grassfeeder-$VERSION ; cat  $DIR/target/grassfeeder-${VERSION}.tar.gz |gzip -d |tar x )

# (cd $WORK/grassfeeder-$VERSION ; cp -v /usr/src/grassfeeder-*.tar.gz . )

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
cat assets/deb-control.txt |egrep  "Package:" |head -n1  >>$CT
cat assets/deb-control.txt |egrep  "Architecture:"  |head -n1  >>$CT
cat assets/deb-control.txt |egrep  "Depends:"  |head -n1  >>$CT
# cat $CT


R="debian/rules"
(cd $WORK/grassfeeder-$VERSION ;   echo "#!/usr/bin/make -f" >$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "clean:" >>$R )
## for ubuntu
# (cd $WORK/grassfeeder-$VERSION ;   echo "	apt update " >>$R )
# (cd $WORK/grassfeeder-$VERSION ;   echo "	apt install -y rust-all cargo   " >>$R )
# (cd $WORK/grassfeeder-$VERSION ;   echo "	apt install -y wget git pkgconf librust-glib-sys-dev libatk1.0-dev librust-gdk-sys-dev libsoup2.4-dev libjavascriptcoregtk-4.0-dev libwebkit2gtk-4.0-dev " >>$R )

# (cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/ ; ./prepare-debian.sh ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; ./unpack-vendored.sh ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-ubuntu/ ; cargo clean ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "build: " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "	(cd grassfeed_rs/app-gtk3-debian/ ; cargo --offline deb ) " >>$R )
(cd $WORK/grassfeeder-$VERSION ;   echo "">>$R )

# (cd $WORK/grassfeeder-$VERSION ;  FAKEROOTKEY=1 LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libfakeroot/libfakeroot-0.so   dpkg-buildpackage -us -uc -ui -S -rfakeroot   )
(cd $WORK/grassfeeder-$VERSION ;   debuild  -rfakeroot -S  )


#echo "----"
#echo dput ppa:schleglermarcus/ppa  grassfeeder_0.1.9_source.changes
##(cd $WORK/grassfeeder-$VERSION ;   echo "	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -sSf | sh -s -- -y	 " >>$R )
##(cd $WORK/grassfeeder-$VERSION ;   echo "	export PATH=/root/.cargo/bin:$PATH " >>$R )
##(cd $WORK/grassfeeder-$VERSION ;   echo "	cargo install cargo-deb " >>$R )

#!/bin/bash

test -d target || mkdir target

for F in src-releases/*.tar.gz ; do
	(cd target/  ; tar xfz ../$F )
done

ls target/

F=target/feed-rs-1.2.0/feed-rs/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"0.25\"/path=\"..\/..\/quick-xml-0.25.0\"/" |egrep -v "regex|url|uuid" 	>$F
echo "regex={ path=\"../../regex-1.6.0\" }  " >>$F
echo "url={ path=\"../../rust-url-2.3.0/url\" }  " >>$F
echo "uuid={ path=\"../../uuid-1.1.0\" , features=[\"v4\"] }  " >>$F


F=target/flume-master/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"0.7\"/path=\"..\/nanorand-rs-0.7.0\"/"  -e "s/\"spin\"/\"spin\" , path=\"..\/spin-rs-0.9.2\" /" 	>$F

F=target/image-0.24.5/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"jpeg-decoder\"/\"jpeg-decoder\", path=\"..\/jpeg-decoder-0.3.0\"/" 	>$F

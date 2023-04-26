#!/bin/bash
D="../target"
test -d $D || mkdir $D
(cd $D ; cat ../app-gtk3-ubuntu/src_vendored.tar.gz |gzip -d |tar x  )

#!/bin/bash
D="../target"
test -d $D || mkdir $D
(cd $D ; cat ../app-gtk3-ubuntu/src_vendored_1.tar.gz |gzip -d |tar x  )
(cd $D ; cat ../app-gtk3-ubuntu/src_vendored_2.tar.gz |gzip -d |tar x  )

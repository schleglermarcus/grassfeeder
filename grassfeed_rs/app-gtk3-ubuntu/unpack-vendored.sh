#!/bin/bash
test -d target || mkdir target
(cd target ; cat ../src_vendored.tar.gz |gzip -d |tar x  )

#!/bin/bash 
git clone https://github.com/gsauthof/pe-util
(cd  pe-util  && git submodule update --init ; mkdir build ; rm -rf .git )
tar c pe-util |gzip >pe_util.tar.gz
rm -rf pe-util





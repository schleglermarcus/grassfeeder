#!/bin/bash
# https://linuxconfig.org/easy-way-to-create-a-debian-package-and-local-package-repository 
#
# dpkg-scanpackages . | gzip -c9  > Packages.gz
dpkg-scanpackages . | gzip  > Packages.gz


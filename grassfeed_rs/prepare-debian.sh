#!/bin/bash
PF="-deb"
for D in  resources ui_gtk  fr_core fr_gtk    ; do
    (cd $D ;  ln -sf Cargo${PF}.toml Cargo.toml  )
done
(cd app-gtk3-debian  ; chmod +x unpack-sources.sh ; ./unpack-sources.sh)

#!/bin/bash
PF="-std"
for D in  resources ui_gtk  fr_core fr_gtk    context  ; do
    (cd $D ;  ln -sf Cargo${PF}.toml Cargo.toml  )
done
test -f Cargo.lock && rm Cargo.lock
rm -rf app-gtk3-debian/target/*

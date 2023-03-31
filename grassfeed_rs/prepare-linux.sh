#!/bin/bash
PF="-std"
for D in  resources ui_gtk  fr_core fr_gtk    ; do
    (cd $D ;  ln -sf Cargo${PF}.toml Cargo.toml  )
done

rm -rf app-gtk3-debian/target/*

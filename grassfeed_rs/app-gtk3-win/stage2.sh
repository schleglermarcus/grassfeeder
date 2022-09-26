#!/bin/bash
rm -rf target
mkdir target
(tar c --exclude=target --exclude=Cargo.lock  ../  |gzip --fast  >target/cr_src.tar.gz )

docker build -t grassfeeder:fedora2 -f fedora-stage2.docker .


# linux,lokale Version	gio v0.15.12	gio-sys v0.15.10 	webkit2gtk v0.18.0
# win, docker version	gio v0.15.12	gio-sys v0.15.10	webkit2gtk v0.18.0


#error[E0412]: cannot find type `UnixFDList` in crate `gio`
#  --> /root/.cargo/registry/src/github.com-1ecc6299db9ec823/webkit2gtk-0.18.0/src/auto/user_message.rs:46:36
#46 |     fd_list: Option<&impl IsA<gio::UnixFDList>>,
#   |                                    ^^^^^^^^^^ not found in `gio`

#error[E0412]: cannot find type `UnixFDList` in crate `gio`
#  --> /root/.cargo/registry/src/github.com-1ecc6299db9ec823/webkit2gtk-0.18.0/src/auto/user_message.rs:85:24
#85 |   fd_list: Option<gio::UnixFDList>,
#   |                        ^^^^^^^^^^ not found in `gio`



docker cp $(docker create --name tc grassfeeder:fedora2):/usr/src/out_package.zip target/ ; docker rm tc
ls -l target/




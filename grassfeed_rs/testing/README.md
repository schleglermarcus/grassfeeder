## AppImage tests

Probing  grassfeeder-0.1.3-B3

| File | Fedora 33 (2020-10) | OpenSuse 15.5 (2022-09) | Ubuntu 20 |
| ---- | ---- | ---- | ---- |
| grassfeeder-0.1.3-B3-fedora33.rpm       |   |   | |
| grassfeeder-0.1.3-B3-fedora33.AppImage  |   |   | |
|   |   |   |   | 
|   |   |   |   | 




### Fedora 33 
\# rpm -i  grassfeeder-0.1.3-B3-fedora33.rpm 
    libatomic.so.1()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libc.so.6(GLIBC_2.32)(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libc.so.6(GLIBC_2.33)(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libc.so.6(GLIBC_2.34)(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libcrypto.so.3()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libffi.so.8()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libicudata.so.71()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libicui18n.so.71()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libicuuc.so.71()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libjpeg.so.62()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libm.so.6(GLIBC_2.35)(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64
    libstemmer.so.0()(64bit) is needed by app-gtk3-linux-0:0.1.3-B3-1.x86_64



### Remarks

     grassfeeder*.AppImage needs at least glibc-2.29
     OpenSuse-15.2 has glibc-2.26

     mint21-built   grassfeeder*.rpm  needs at least glibc-2.32
     OpenSuse-15.3  has glibc-2.31
     OpenSuse-15.4  has glibc-2.31
     OpenSuse-15.5  has glibc-2.31
     
     Fedora-29  has glibc-2.28.9, too old
     Fedora-30  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.32
     Fedora-31  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.32,  only 2.30 is available
     Fedora-33  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.33,  only 2.32 is available
                       



### End of Life: 
[Ubuntu](https://endoflife.date/ubuntu) [Suse](https://endoflife.date/opensuse) [Fedora](https://endoflife.date/fedora)




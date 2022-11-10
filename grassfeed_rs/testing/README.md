## Distribution test matrix: 


| File                                      | Fedora 35 (2021-11) | OpenSuse 15.5 (2022-09) | Ubuntu 20 |
| ---- | ---- | ---- | ---- |
| grassfeeder-0.1.3-B5-fedora35.rpm         | yes   | no  | -   |
| grassfeeder-0.1.3-B5-fedora35.AppImage    | yes   | no  | no  |
| grassfeeder-0.1.3-B5-suse154.rpm          | yes   | yes | -   |
| grassfeeder-0.1.3-B4-suse154.AppImage     | no    | yes | no  |
| grassfeeder-0.1.3-B7-debian11.rpm         | no    | no  | -   |
| grassfeeder-0.1.3-B7-debian11.AppImage    | no    | no  | yes |
| grassfeeder-0.1.3-B3-mint20.AppImage      | no    | no  | yes |  

### running Fedora 35

` dnf install -y libatomic  `  
` rpm -i grassfeeder-0.1.3-B5-fedora35.rpm `   &#x2714;

```

./grassfeeder-0.1.3-B7-debian11.AppImage 
./grassfeeder-0.1.3-B7-debian11.AppImage: /lib64/libpthread.so.0: version `GLIBC_PRIVATE' not found (required by /tmp/.mount_grassfAbmqKo/lib/x86_64-linux-gnu/librt.so.1)

[marcus@fedora gfdist]$ rpm -i grassfeeder-0.1.3-B7-debian11.rpm 
Fehler: Fehlgeschlagene Abhängigkeiten:
    libbz2.so.1.0()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libffi.so.7()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libicudata.so.67()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libicui18n.so.67()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libicuuc.so.67()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libmd.so.0()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libpcre.so.3()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64
    libwebp.so.6()(64bit) wird benötigt von grassfeeder-0:0.1.3-B7-1.x86_64


# ./grassfeeder-0.1.3-B4-suse154.AppImage
./grassfeeder-0.1.3-B4-suse154.AppImage: /lib64/libpthread.so.0: version `GLIBC_PRIVATE' not found (required by /tmp/.mount_grassf7s2IXB/lib64/librt.so.1)


# ./grassfeeder-0.1.3-B3-mint20.AppImage 
./grassfeeder-0.1.3-B3-mint20.AppImage: /lib64/libpthread.so.0: version `GLIBC_PRIVATE' not found (required by /tmp/.mount_grassflDsXN6/lib/x86_64-linux-gnu/librt.so.1)
```

#### running Fedora 33 or older
     Fedora-29  has glibc-2.28.9, too old
     Fedora-30  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.32
     Fedora-31  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.32,  only 2.30 is available
     Fedora-33  :  *.AppImage:    web_view_get_is_web_process_responsive  not found -> webkit2gtk too old
                  *.rpm   needs   glibc-2.33,  only 2.32 is available
                       


### running  OpenSuse 15.5
`zypper install libatomic1  `   &#x2714;

 - Present:  glibc-2.31    
 - Needed by grassfeeder-0.1.3-B3-fedora35.rpm :  glibc-2.32

```
# rpm -i grassfeeder-0.1.3-B5-fedora35.rpm 
error: Failed dependencies:
    libc.so.6(GLIBC_2.32)(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    libc.so.6(GLIBC_2.33)(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    libc.so.6(GLIBC_2.34)(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    libffi.so.6()(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    libjpeg.so.62()(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    libstemmer.so.0()(64bit) is needed by app-gtk3-linux-0:0.1.3-B5-1.x86_64
    
    
# ./grassfeeder-0.1.3-B7-debian11.AppImage 
(grassfeeder-0.1.3-B7-debian11.AppImage:2769): Gtk-WARNING **: 16:08:05.379: Could not load a pixbuf from icon theme.
This may indicate that pixbuf loaders or the mime database could not be found.
**
Gtk:ERROR:../../../../gtk/gtkiconhelper.c:494:ensure_surface_for_gicon: assertion failed (error == NULL): Failed to load /usr/share/icons/Adwaita/16x16/status/image-missing.png: Format der Bilddatei unbekannt (gdk-pixbuf-error-quark, 3)
Bail out! Gtk:ERROR:../../../../gtk/gtkiconhelper.c:494:ensure_surface_for_gicon: assertion failed (error == NULL): Failed to load /usr/share/icons/Adwaita/16x16/status/image-missing.png: Format der Bilddatei unbekannt (gdk-pixbuf-error-quark, 3)
Abgebrochen (Speicherabzug geschrieben)


# ./grassfeeder-0.1.3-B3-mint20.AppImage 
(grassfeeder-0.1.3-B3-mint20.AppImage:3831): GLib-GIO-ERROR **: 18:17:38.946: Settings schema 'org.gnome.settings-daemon.plugins.xsettings' does not contain a key named 'antialiasing'
Trace/Breakpoint ausgelöst (Speicherabzug geschrieben)


# rpm -i grassfeeder-0.1.3-B7-debian11.rpm 
error: Failed dependencies:
    libbz2.so.1.0()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libicudata.so.67()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libicui18n.so.67()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libicuuc.so.67()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libjpeg.so.62()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libmd.so.0()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libpcre.so.3()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64
    libwebp.so.6()(64bit) is needed by grassfeeder-0:0.1.3-B7-1.x86_64

    
```

#### OpenSuse  15.4 and older
     OpenSuse-15.5  has glibc-2.31
     OpenSuse-15.4  has glibc-2.31
     OpenSuse-15.3  has glibc-2.31
     OpenSuse-15.2  has glibc-2.26

    grassfeeder-0.1.3-B3-mint20.AppImage needs at least glibc-2.32


### running Ubuntu20
```
#  ./grassfeeder-0.1.3-B5-fedora35.AppImage 
./grassfeeder-0.1.3-B5-fedora35.AppImage: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.33' not found (required by ./grassfeeder-0.1.3-B5-fedora35.AppImage)
./grassfeeder-0.1.3-B5-fedora35.AppImage: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.32' not found (required by ./grassfeeder-0.1.3-B5-fedora35.AppImage)
./grassfeeder-0.1.3-B5-fedora35.AppImage: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.34' not found (required by ./grassfeeder-0.1.3-B5-fedora35.AppImage)

# ./grassfeeder-0.1.3-B4-suse154.AppImage 
./grassfeeder-0.1.3-B4-suse154.AppImage: symbol lookup error: /tmp/.mount_grassfurPRMl/lib64/librt.so.1: undefined symbol: __pthread_attr_copy, version GLIBC_PRIVATE



```     



### End of Life: 
[Ubuntu](https://endoflife.date/ubuntu) [Suse](https://endoflife.date/opensuse) [Fedora](https://endoflife.date/fedora)




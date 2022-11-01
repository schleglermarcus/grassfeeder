## Installation


| OS  | *.deb  | *.AppImage |
| ---- | ---- | ---- |
| Linux Mint 21  | &#x2714;  | &#x2714;  | 
| Linux Mint 20  | &#x274C;  | &#x2714;  | 
| Ubuntu 20      | &#x2714;  | &#x2714;  | 
| Ubuntu 18      | &#x274C;  | &#x274C;  | 



### *.deb
* Download the grassfeeder_*_amd64.deb from the releases page
* Either Use the file manager nemo, the "debi" program
* Or `sudo dpkg -i  grassfeeder_0.0.4_amd64.deb`


### *.AppImage
* Download the grassfeeder_*.AppImage from the releases page
* chmod +x grassfeeder_*.AppImage
* ./grassfeeder_*.AppImage




## Dependencies
#### Build

##### Ubuntu 18.04
 - apt-get install  curl git gcc  pkg-config
 - The package glib-sys-0.15.10  requires  glibc >2.48
 - \# pkg-config --libs --cflags glib-2.0 "glib-2.0 >= 2.48"
      Package glib-2.0 was not found in the pkg-config search path.
 - This [version](https://distrowatch.com/table.php?distribution=ubuntu) 18.04 is too old 

##### Linux Mint 20 preparation, as user

Version 0.0.4 and down do not work here, since they have no libgdk-pixbuf-2.0-0:amd64
They use the old package "libgdk-pixbuf2.0-0:amd64"  and do not have an easy upgrade path.
###### Mint20  uses
- "focal" for Ubuntu packages
- "uma" for Mint packages
###### Mint21 uses
- "jammy" for Ubuntu packages
- "vanessa" for Mint  packages

  - Install rust:  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
	- `cargo install cargo-deb`

##### Linux Mint 20 preparation, as admin
  - apt-get install git
  - apt-get install libsoup2.4-dev
	- apt-get install librust-gdk-sys-dev
	- apt-get install libjavascriptcoregtk-4.0-dev
	- apt-get install libwebkit2gtk-4.0-dev




#

#### Binary
- libglib2.0-0 (>= 2.33.14)
- libgtk-3-0 (>= 3.16.2)
- libwebkit2gtk-4.0-37 (>= 2.10.0)
- libc6 (>= 2.35)
- libgdk-pixbuf-2.0-0 (>= 2.22.0)
- libpango-1.0-0 (>= 1.14.0)




### Linux Mint 20



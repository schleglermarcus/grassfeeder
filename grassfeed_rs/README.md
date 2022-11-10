## Installation


| OS  | *.deb  | *.AppImage | *.rpm |
| ---- | ---- | ---- | ---- |
| Linux Mint 21             | &#x2714;  | &#x2714;  | -        | 
| Linux Mint 20             | &#x274C;  | &#x2714;  | -        |
| Ubuntu 20                 | &#x2714;  | &#x2714;  | -        |
| Ubuntu 18                 | &#x274C;  | &#x274C;  | -        |
| OpenSuse 15.5 (2022-09)   | -         | &#x2714;  | &#x274C; | 
| OpenSuse 15.4 (2021-12)   | -         | &#x2714;  | &#x274C; | 
| OpenSuse 15.3 (2021-05)   | -         | &#x2714;  | &#x274C; | 
| OpenSuse 15.2 (2021-02)   | -         | &#x274C;  | &#x274C; |
| Fedora 35 (2021-11)       | -         | &#x2714;  | &#x2714; |
| Fedora 33 (2020-10)       | -         | &#x274C;  |          |
| Fedora 31 (2019-10)       | -         | &#x274C;  | &#x274C; |
| Fedora 30 (2019-04)       | -         | no        | no       |



End of Life: [Ubuntu](https://endoflife.date/ubuntu) [Suse](https://endoflife.date/opensuse) [Fedora](https://endoflife.date/fedora)

### *.rpm
Requisite for Fedora 35: `dnf install -y libatomic`

Requisite for OpenSuse 15.5: `zypper install  libatomic1`


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
 - apt-get install -y  curl git gcc  pkg-config  libglib2.0-dev  libatk1.0-dev  libgdk-pixbuf2.0-dev   libpango1.0-dev
 - apt-get install -y  libgdk3.0-cil-dev  libsoup2.4-dev libcairo2-dev libjavascriptcoregtk-4.0-dev  libgtk-3-dev
 
 
 - The rust package glib-sys-0.15.10  requires  glibc >2.48
 - \# pkg-config --libs --cflags glib-2.0 "glib-2.0 >= 2.48"
 ```
  cargo:warning=`"pkg-config" "--libs" "--cflags" "gtk+-3.0" "gtk+-3.0 >= 3.24.8"` did not exit successfully: exit status: 1
  error: could not find system library 'gtk+-3.0' required by the 'gtk-sys' crate
  Requested 'gtk+-3.0 >= 3.24.8' but version of GTK+ is 3.22.30
 ```
 
 
 
 
 
 
      Package glib-2.0 was not found in the pkg-config search path.
 - This [version](https://distrowatch.com/table.php?distribution=ubuntu) 18.04 is too old 

##### Fedora 33 
Too old,  we need  rust-atk-sys+default-devel, which is only available on  fedora 35 and up


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


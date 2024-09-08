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

##### Linux Mint
 Mint 22 : Needs libsoup-3.0-dev     libjavascriptcoregtk-4.1-dev   libwebkit2gtk-4.1-dev


##### Ubuntu 18.04
 - apt-get install -y  curl git gcc  pkg-config  libglib2.0-dev  libatk1.0-dev  libgdk-pixbuf2.0-dev   libpango1.0-dev
 - apt-get install -y  libgdk3.0-cil-dev  libsoup2.4-dev libcairo2-dev libjavascriptcoregtk-4.0-dev  libgtk-3-dev
 - apt-get install -y  libwebkit2gtk-4.0-dev

    Finally, the linker fails with webkit2gtk.

 - This [version](https://distrowatch.com/table.php?distribution=ubuntu) 18.04 is too old

##### Fedora 33
Too old,  we need  rust-atk-sys+default-devel, which is only available on  fedora 35 and up

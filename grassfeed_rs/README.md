## Installation


### Linux Mint 21
* Fetch the grassfeeder_*_amd64.deb from the releases page
* Either Use the file manager nemo, the "debi" program
* Or `sudo dpkg -i  grassfeeder_0.0.4_amd64.deb`

### Linux Mint 20
Version 0.0.4 and down do not work here, since they have no libgdk-pixbuf-2.0-0:amd64  
They use the old package "libgdk-pixbuf2.0-0:amd64"  and do not have an easy upgrade path. 
###### Mint20  uses 
- "focal" for Ubuntu packages
- "uma" for Mint packages
###### Mint21 uses 
- "jammy" for Ubuntu packages
- "vanessa" for Mint  packages

    




## Dependencies
#### Build 

* ~~apt-get install libjavascriptcoregtk-4.0-dev~~
* ~~apt-get install libsoup2.4-dev~~
* ~~apt-get install libssl-dev~~ 

#### Binary
- libglib2.0-0 (>= 2.33.14)
- libgtk-3-0 (>= 3.16.2)
- libwebkit2gtk-4.0-37 (>= 2.10.0)
- libc6 (>= 2.35)
- libgdk-pixbuf-2.0-0 (>= 2.22.0)
- libpango-1.0-0 (>= 1.14.0)



## Package Source for  Debian / Ubuntu / Mint


1. Add the key file to your local  mint / debian : 
`wget -P /usr/share/keyrings  https://github.com/schleglermarcus/grassfeeder/raw/main/debian/grassfeeder-archive-keyring.gpg  `


2. Add this repository to the sources list, either via   Settings / Package Sourcs, or via   /etc/apt/sources.list.d/additional-repositories.list
`deb [arch=amd64 signed-by=/usr/share/keyrings/grassfeeder-archive-keyring.gpg] https://raw.githubusercontent.com/schleglermarcus/grassfeeder/debian/ grassfeeder main`

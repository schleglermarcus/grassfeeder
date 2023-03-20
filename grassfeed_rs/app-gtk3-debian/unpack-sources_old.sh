#!/bin/bash
test -d target || mkdir target

for F in src-releases/*.tar.?? ; do
	(cd target/  ; tar xf ../$F  )
done

ls target/

F=target/feed-rs-1.2.0/feed-rs/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"0.25\"/path=\"..\/..\/quick-xml-0.25.0\"/" |egrep -v "regex|url|uuid" 	>$F
echo "regex={ path=\"../../regex-1.6.0\" }  " >>$F
echo "url={ path=\"../../rust-url-2.3.0/url\" }  " >>$F
echo "uuid={ path=\"../../uuid-1.1.0\" , features=[\"v4\"] }  " >>$F


F=target/flume-master/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"0.7\"/path=\"..\/nanorand-rs-0.7.0\"/"  -e "s/\"spin\"/\"spin\" , path=\"..\/spin-rs-0.9.2\" /" 	>$F

F=target/image-0.24.5/Cargo.toml
mv $F ${F}.0			# downgrading color_quant, gif , png, tiff
cat ${F}.0 |sed -e "s/\"jpeg-decoder\"/\"jpeg-decoder\", path=\"..\/jpeg-decoder-0.3.0\"/" \
 		|sed -e "s/\"1.7.0\"/\"1.7.0\", path=\"..\/bytemuck-1.7.0\" /"  \
 		|sed -e "s/\"1.1\"/\"1.0\" /" \
    |sed -e "s/\"1.5.0\"/\"1.5.0\", path=\"..\/exrs-1.5.0\"   /" \
		|sed -e "s/\"0.11.1\"/\">=0.10.0\"/" \
		|sed -e "s/tiff = { version = \"0.8.0\"/tiff={version=\">=0.5\"/" \
		|sed -e "s/\"0.17.6\"/\"0.17.5\", path=\"..\/image-png-0.17.5\"   /" \
		>$F

F=target/rust-ico-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.17\"/\{version=\"0.17\", path=\"..\/image-png-0.17.5\"\}/" 	>$F

F=target/exrs-1.5.0/Cargo.toml
mv $F ${F}.0		# downgrading miniz_oxide		lebe
cat ${F}.0 |sed -e "s/\"\^0.10.1\"/\{version=\"\^0.10.1\", path=\"..\/rust-bit-field-0.10.1\"\}/" \
	|sed -e "s/\"\^0.10.9\"/\{version=\"\^0.10.9\", path=\"..\/flume-master\"\}/" \
	|sed -e "s/\"\^1.8.2\"/\{version=\"\^1.8.2\", path=\"..\/half-rs-1.8.2\"\}/" \
	|sed -e "s/\"\^0.5.2\"/\{version=\">=0.5.0\", path=\"..\/lebe-0.5.2\"\}/" \
	|sed -e "s/\"\^0.5.3\"/\">=0.4.0\"/" \
	|sed -e "s/\"\^1.8.1\"/\">=1.7.0\"/" \
	>$F

F=target/jpeg-decoder-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"1.5.1\"/version=\">=1.5.1\", path=\"..\/rayon-1.5.1\"    /" 	>$F


F=target/libwebp-image-rs-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.24.0\"/\">=0.24.0\", path=\"..\/image-0.24.5\" /" \
	|sed -e "s/\"0.1.0\"/{ version=\">=0.1.0\", path=\"..\/libwebp-rs-0.1.0\" }/" \
	>$F

F=target/libwebp-rs-0.1.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/libwebp-sys2 = \"0.1.0\"/libwebp-sys2 ={version=\"0.1.2\", path=\"..\/libwebp-sys2-rs-0.1.2\"} /" >$F

F=target/nanorand-rs-0.7.0/Cargo.toml
mv $F ${F}.0			#  getrandom downgrade , removing   js feature
cat ${F}.0 |sed -e "s/\"0.2.5\"/\">=0.2.4\" /"  \
	|sed -e "s/, \"js\"//"	\
 	>$F

F=target/image-png-0.17.5/Cargo.toml
mv $F ${F}.0    #  downgrading deflate, miniz_oxide
cat ${F}.0 |sed -e "s/deflate = \"1.0\"/deflate=\{version=\">=0.7.0\" } /" 	>$F
#	|sed -e "s/\"\0.5.1\"/\">=0.4.0\"/" \

F=target/quick-xml-0.25.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"2.5\"/\">=2.4\" /" 		>$F


F=target/regex-1.6.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.7.18\"/\">=0.7.10\" /" 		>$F

F=target/rusqlite-sys0.25.2/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/hashlink = \"0.8\"/hashlink=\{version=\">=0.7\" , path=\"..\/hashlink-0.8.0\"\}  /" 		>$F


F=target/hashlink-0.8.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.12.0\"/\">=0.11\" /" 		>$F

F=target/rust-i18n-1.1.1/Cargo.toml
mv $F ${F}.0		#downgrading itertools, once_cell
cat ${F}.0 |sed -e "s/\"0.10.3\"/\">=0.10.0\" /" 	 |sed -e "s/\"1.10.0\"/\">=1.9\" /" 		>$F

F=target/rust-i18n-1.1.1/crates/macro/Cargo.toml
mv $F ${F}.0		# once_cell, syn
cat ${F}.0  |sed -e "s/\"1.10.0\"/\">=1.9\" /"  |sed -e "s/\"1.0.82\"/\">=1.0.76\" /" 		>$F


F=target/rust-i18n-1.1.1/crates/support/Cargo.toml
mv $F ${F}.0		# once_cell
cat ${F}.0  |sed -e "s/\"1.10.0\"/\">=1.9\" /"  		>$F


F=target/embedded-graphics-embedded-graphics-v0.7.1/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"1.1\"/\{version=\">=1.1\", path=\"..\/az-v1.1.0\"\} /" \
	|sed -e "s/\"1.1.0\"/\">=1.1.0\", path=\"..\/micromath-1.1.1\" /" \
	|sed -e "s/\"0.8.0\"/\">=0.6.0\" /" 			>$F

F=target/embedded-graphics-embedded-graphics-v0.7.1/core/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"1.1\"/\{version=\">=1.1\", path=\"..\/..\/az-v1.1.0\"\} /" 		>$F


F=target/tinybmp-0.4.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.7.1\"/\{version=\">=0.7.1\", path=\"..\/embedded-graphics-embedded-graphics-v0.7.1\" \} /"		>$F


F=target/rustls-v-0.20.8/rustls/Cargo.toml
mv $F ${F}.0	# downgrading  ring, sct, webpki		# removing feature  "alloc"
cat ${F}.0 |sed -e "s/\"0.16.20\"/\">=0.16.9\" /"		\
	|sed -e "s/\"0.7.0\"/\">=0.6.0\" /"		\
	|sed -e "s/\"0.22.0\"/\">=0.21.0\" /"		\
	|sed -e "s/\"alloc\",//"		\
	>$F

F=target/ureq-2.6.2/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/rustls = { version = \"/rustls = \{path=\"..\/rustls-v-0.20.8\/rustls\", version=\">=/"		\
		|sed -e "s/url = \"2\"/url=\{version=\">=2.0\", path=\"..\/rust-url-2.3.0\/url\"\} /"	\
		|sed -e "s/webpki = { version = \"0.22\"/webpki=\{version=\">=0.21\"  /"	\
		|sed -e "s/webpki-roots = {/\webpki-roots={path=\"..\/webpki-roots-v-0.22.6\"\,  /"	\
	>$F

F=target/resvg-0.29.0/usvg/Cargo.toml
mv $F ${F}.0		# downgrading base64 ,  data-url  imagesize   kurbo		rctree	strict-num
cat ${F}.0 	|sed -e "s/\"0.21\"/\">=0.13\" /"\
		|sed -e "s/\"0.2\"/\">=0.1\" /"	\
		|sed -e "s/\"0.11\"/\{version=\"0.11\", path=\"..\/..\/imagesize-0.11.0\"\}  /" \
		|sed -e "s/\"0.9\"/\">=0.7\" /"	\
		|sed -e "s/\"0.5\"/\">=0.3\" /"	\
		|sed -e "s/strict-num = \"0.1\"/strict-num={version=\">=0.1\", path=\"..\/..\/strict-num-0.1.0\"\}  /" \
				>$F

F=target/resvg-0.29.0/rosvgtree/Cargo.toml
mv $F ${F}.0		# downgrading roxmltree, svgtypes
cat ${F}.0 	|sed -e "s/\"0.18\"/\">=0.7\" /"  \
	| sed -e "s/\"0.10\"/\">=0.5\" /"	\
						>$F

F=target/strict-num-0.1.0/Cargo.toml
mv $F ${F}.0		#  downgrade of  float-cmp,   removing feature  "std"
cat ${F}.0 |sed -e "s/\"0.9\"/\">=0.6\" /" |sed -e "s/\"std\"//"	>$F

F=target/webpki-roots-v-0.22.6/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.22.0\"/\">=0.21.0\" /"	>$F

F=target/xmlem-0.2.0/Cargo.toml
mv $F ${F}.0		# downgrading   indexmap, once_cell, selectors, thin-slice, slotmap, unic-ucd
cat ${F}.0 	|sed -e "s/\"0.28.1\"/{version=\">=0.28\", path=\"..\/rust-cssparser-0.28.0\"\}  /" \
	|sed -e "s/\"1.8.1\"/\">=1.7.0\" /"	\
	|sed -e "s/\"1.10.0\"/\">=1.9.0\" /" \
	|sed -e "s/\"0.1.0\"/\{version=\"0.1.0\", path=\"..\/qname-0.1.0\"\}  /" \
	|sed -e "s/\"0.26.0\"/\{version=\">=0.25.0\", path=\"..\/quick-xml-0.25.0\"\}  /" \
	|sed -e "s/\"0.23.0\"/{version=\">=0.22.0\", path=\"..\/servo-selectors-v0.22.0\/components\/selectors\"\}  /"	\
	|sed -e "s/\"1.0.6\"/{version=\">=1.0.6\", path=\"..\/slotmap-1.0.6\"\}  /"	\
	|sed -e "s/\"0.9.0\"/{version=\"0.9.0\", path=\"..\/rust-unic-0.9.0\/unic\/ucd\" \}  /"	\
	>$F

F=target/servo-selectors-v0.22.0/components/selectors/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.27\"/\{version=\">=0.27\" , path=\"..\/..\/..\/rust-cssparser-0.28.0\"  \}/"	\
 	|sed -e "s/\"0.99\"/\{version=\">=0.99\" , path=\"..\/..\/..\/derive_more-0.99.17\"  \}/"	\
	|sed -e "s/\"0.1.0\"/\">=0.1.0\" /"	\
	>$F

F=target/convert_case-0.6.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"1.9.0\"/\">=1.6.0\" /"	>$F

F=target/derive_more-0.99.17/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/convert_case = { version = \"0.4\"/convert_case=\{version=\">=0.4\", path=\"..\/convert_case-0.6.0\"\   /"	>$F

F=target/hard-xml-v1.19.0/hard-xml/Cargo.toml
mv $F ${F}.0		#  restoring the version number  here!    env_logger
cat ${F}.0 |sed -e "s/\"0.5\"/\{version=\">=0.5\" , path=\"..\/..\/jetscii-0.5.3\"  \}/" \
	|sed -e "s/\"0.0.0\"/\"1.19.0\"/" \
	|sed -e "s/\"0.13\"/\">=0.11.0\" /" \
	|sed -e "s/\"0.8\"/\">=0.8\" /" \
	>$F
rm target/hard-xml-v1.19.0/rust-toolchain.toml

F=target/hard-xml-v1.19.0/hard-xml-derive/Cargo.toml
mv $F ${F}.0		#  restoring the version number  here!
cat ${F}.0 		|sed -e "s/\"0.0.0\"/\"1.19.0\"/" 	>$F

F=target/hard-xml-v1.19.0/test-suite/Cargo.toml
mv $F ${F}.0		#   env_logger
cat ${F}.0 		|sed -e "s/\"0.8\"/\">=0.8\" /"	|sed -e "s/\"1.0.71\"/\">=1.0.50\" /" 	>$F

F=target/opml/opml_api/Cargo.toml
mv $F ${F}.0	#   serde , thiserror
cat ${F}.0  |sed -e "s/\"1.13.0\"/\{version=\">=1.11\" , path=\"..\/..\/hard-xml-v1.19.0\/hard-xml\"  \} /" 	\
	|sed -e "s/\"1.0.145\"/\">=1.0.130\"/" \
	|sed -e "s/\"1.0.37\"/\">=1.0.20\"/" \
	>$F

F=target/rust-url-2.3.0/idna/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.1.17\"/\">=0.1.12\"/"	>$F

F=target/webkit2gtk-rs-webkit2gtk-rs-v0.16.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/javascriptcore-rs = \"^0.15.2\"/javascriptcore-rs=\{ version=\">=0.14.9\", path=\"..\/javascriptcore-rs-javascriptcore-rs-v0.15.2\" \} /" \
	>$F

F=target/webkit2gtk-rs-webkit2gtk-rs-v0.16.0/sys/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/javascriptcore-rs-sys = \"^0.3.2\"/javascriptcore-rs-sys=\{ version=\">=0.3.2\", path=\"..\/..\/javascriptcore-rs-javascriptcore-rs-v0.15.2\/sys\" \} /" \
		|sed -e "s/soup2-sys = \"^0.1.0\"/soup2-sys=\{ version=\">=0.1.0\", path=\"..\/..\/soup2-sys-0.2.0\" \} /" \
    |sed -e "s/system-deps = \"5\"/system-deps=\">=3\"/" \
		>$F

F=target/javascriptcore-rs-javascriptcore-rs-v0.15.2/sys/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/system-deps = \"5\"/system-deps=\">=3\"/" >$F


F=target/soup2-sys-0.2.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.15\"/\">=0.14\"/g" \
 	|sed -e "s/= \"5\"/=  \">=3\"/"  \
	>$F

F=target/rayon-1.5.1/Cargo.toml
mv $F ${F}.0	# downgrade   crossbeam-channel		crossbeam-deque
cat ${F}.0 	 |sed -e "s/\"0.8.1\"/\">=0.7.4\"/"	\
		>$F

F=target/rayon-1.5.1/rayon-core/Cargo.toml
mv $F ${F}.0	# downgrade   crossbeam-channel		crossbeam-deque
cat ${F}.0 |sed -e "s/\"0.5.0\"/\">=0.4\"/" 	\
	 |sed -e "s/\"0.8.1\"/\">=0.7.4\"/"	\
	>$F



#----------------------------------------------------------
exit




F=target/webkit2gtk-rs-webkit2gtk-rs-v0.18.2/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/cairo-rs = \"^0.15.0\"/cairo-rs=\">=0.14.9\"/" \
	|sed -e "s/gdk = \"^0.15.0\"/gdk=\">=0.14.3\"/" \
	|sed -e "s/gdk-sys = \"^0.15.0\"/gdk-sys=\">=0.14.0\"/" \
	|sed -e "s/gio = \"^0.15.0\"/gio=\">=0.14.8\"/" \
	|sed -e "s/gio-sys = \"^0.15.0\"/gio-sys=\">=0.14.0\"/" \
	|sed -e "s/glib = \"^0.15.0\"/glib=\">=0.14.8\"/" \
	|sed -e "s/glib-sys = \"^0.15.0\"/glib-sys=\">=0.14.0\"/" \
	|sed -e "s/gobject-sys = \"^0.15.0\"/gobject-sys=\">=0.14.0\"/" \
	|sed -e "s/gtk = \"^0.15.0\"/gtk=\">=0.14.3\"/" \
	|sed -e "s/gtk-sys = \"^0.15.0\"/gtk-sys=\">=0.14.0\"/" \
	>$F

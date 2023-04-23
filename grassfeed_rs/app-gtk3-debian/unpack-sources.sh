#!/bin/bash
test -d target || mkdir target

for F in src-releases/*.tar.?? ; do
	(cd target/  ; tar xf ../$F  )
done

# ls target/

F=target/feed-rs-1.3.0/feed-rs/Cargo.toml
mv $F ${F}.0	#  downgrading chrono, quick-xml
cat ${F}.0 |sed -e "s/\"0.4.23\"/\"0.4.24\", path=\"..\/..\/chrono-0.4.24\"  /"  \
	|sed -e "s/\"0.27.1\"/\"0.27.1\",  path=\"..\/..\/quick-xml-0.27.1\"/" \
	|sed -e "s/serde_json = \"1.0\"/serde_json={version=\">=1\" , path=\"..\/..\/json-1.0.93\" }  /" 	\
	|sed -e "s/serde = { version = \"1.0\"/serde={version=\">=1\" , path=\"..\/..\/serde-1.0.156\/serde\"   /" 	\
	|egrep -v "regex|url|uuid" 		>$F
echo "regex={ path=\"../../regex-1.6.0\" }  " >>$F
echo "url={ path=\"../../rust-url-2.3.0/url\" }  " >>$F
echo "uuid={ path=\"../../uuid-1.1.0\" , features=[\"v4\"] }  " >>$F


F=target/flume-master/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"0.7\"/path=\"..\/nanorand-rs-0.7.0\"/"  -e "s/\"spin\"/\"spin\" , path=\"..\/spin-rs-0.9.2\" /" 	>$F


F=target/image-0.24.0/Cargo.toml	# upgrading  num-iter, num-rational, exr
mv $F ${F}.0			# downgrading   jpeg_decoder color_quant, gif , png, tiff,
cat ${F}.0 |sed -e "s/\"jpeg-decoder\", version = \"0.2.1\"/\"jpeg-decoder\", version=\">=0.2.1\", path=\"..\/jpeg-decoder-0.3.0\"/" \
		|sed -e "s/\"0.1.22\"/\">=0.1\"  /"  \
		|sed -e "s/\"0.11.1\"/\">=0.11.1\", path=\"..\/image-gif-0.12.0\"     /"  \
 		|sed -e "s/bytemuck = { version = \"1.7.0\"/bytemuck={version=\">=1\", path=\"..\/bytemuck-1.7.0\"  /"  \
 		|sed -e "s/\"1.1\"/\"1.0\" /" \
    	|sed -e "s/\"1.4.1\"/\">=1.4.1\", path=\"..\/exrs-1.5.0\"   /" \
		|sed -e "s/tiff = { version = \"0.7.1\"/tiff={version=\">=0.6\" , path=\"..\/image-tiff-0.8.0\" /" \
		|sed -e "s/\"0.17.0\"/\">=0.16.5\", path=\"..\/image-png-0.17.5\"   /" \
 		|sed -e "s/\"0.1.32\"/\">=0.1.32\" /" \
 		|sed -e "s/\"0.3\"/\">=0.3\" /" \
		>$F


F=target/rust-ico-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.17\"/\{version=\">=0.16\", path=\"..\/image-png-0.17.5\"\}/" 	>$F

F=target/exrs-1.5.0/Cargo.toml
mv $F ${F}.0		# downgrading miniz_oxide		lebe
cat ${F}.0 |sed -e "s/\"\^0.10.1\"/\{version=\"\^0.10.1\", path=\"..\/rust-bit-field-0.10.1\"\}/" \
	|sed -e "s/\"\^0.10.9\"/\{version=\"\^0.10.9\", path=\"..\/flume-master\"\}/" \
	|sed -e "s/\"\^1.8.2\"/\{version=\"\^1.8.2\", path=\"..\/half-rs-1.8.2\"\}/" \
	|sed -e "s/\"\^0.5.2\"/\{version=\">=0.5.0\", path=\"..\/lebe-0.5.2\"\}/" \
	|sed -e "s/\"\^0.5.3\"/\">=0.4.0\"/" \
	|sed -e "s/\"\^1.8.1\"/{version=\">=1.7.0\" , path=\"..\/rust-threadpool-1.8.1\" \} /" \
	>$F
# |sed -e "s/\"\0.5.1\"/{ version=\">=0.5.1\" , path=\"..\/miniz_oxide-0.6.2\/miniz_oxide\" } /" \


F=target/jpeg-decoder-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/version = \"1.5.1\"/version=\">=1.5.1\", path=\"..\/rayon-1.5.1\"    /" 	>$F


F=target/libwebp-image-rs-0.3.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.24.0\"/\">=0.23.0\", path=\"..\/image-0.24.0\" /" \
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
cat ${F}.0	| sed -e "s/deflate = \"1.0\"/deflate={version=\"1.0\", path=\"..\/deflate-1.0.0\"} /"	\
	| sed -e "s/\"\0.5.1\"/{ version=\">=0.5.1\" , path=\"..\/miniz_oxide-0.5.1\/miniz_oxide\" } /"  \
	>$F

F=target/quick-xml-0.27.1/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"2.5\"/\">=2.4\" /" 		>$F


F=target/regex-1.6.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.7.18\"/\">=0.7.10\" /" 		>$F

F=target/rusqlite-0.28.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/hashlink = \"0.8\"/hashlink=\{version=\">=0.7\" , path=\"..\/hashlink-0.8.0\"\}  /" 		>$F

F=target/hashlink-0.8.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.12.0\"/{version=\"^0.12.0\", path=\"..\/hashbrown-0.12.0\" }/" 		>$F

F=target/rust-i18n-1.1.1/Cargo.toml
mv $F ${F}.0		#downgrading itertools, once_cell, upgrading clap, anyhow, regex
cat ${F}.0 \
	|sed -e "s/anyhow = {version = \"1\"/anyhow={version=\">=1\", path=\"..\/anyhow-1.0.60\"   /" 	\
	|sed -e "s/\"0.10.3\"/\">=0.10.0\"  , path=\"..\/itertools-0.10.3\"  /" 	 \
	|sed -e "s/\"1.10.0\"/{version=\">=1.3.1\" , path=\"..\/once_cell-1.10.0\" }  /" 	\
	|sed -e "s/\"2.32\"/\">=2.32\", path=\"..\/clap-3.2.20\"   /" 	\
	|sed -e "s/serde = \"1\"/serde={version=\">=1.0.130\" , path=\"..\/serde-1.0.156\/serde\" }  /" 	\
	|sed -e "s/serde_derive = \"1\"/serde_derive={version=\">=1.0.130\" , path=\"..\/serde-1.0.156\/serde_derive\" }  /" 	\
	|sed -e "s/regex = \"1\"/regex={version=\">=1\" , path=\"..\/regex-1.6.0\" }  /" 	\
	>$F


F=target/rust-i18n-1.1.1/crates/macro/Cargo.toml
mv $F ${F}.0		# once_cell, syn
cat ${F}.0  |sed -e "s/\"1.10.0\"/\">=1.3.1\" /"  |sed -e "s/\"1.0.82\"/\">=1.0.76\" /"		\
	|sed -e "s/serde_json = \"1\"/serde_json={version=\">=1\" , path=\"..\/..\/..\/json-1.0.93\" }  /" 	\
	>$F

F=target/rust-i18n-1.1.1/crates/support/Cargo.toml
mv $F ${F}.0		# once_cell, serde_json
cat ${F}.0  |sed -e "s/\"1.10.0\"/\">=1.3.1\" /"  	\
	|sed -e "s/serde_json = \"1\"/serde_json={version=\">=1\" , path=\"..\/..\/..\/json-1.0.93\" }  /" 	\
	>$F

F=target/rust-i18n-1.1.1/crates/extract/Cargo.toml
mv $F ${F}.0		#  serde_json
cat ${F}.0  	\
	|sed -e "s/serde_json = \"1\"/serde_json={version=\">=1\" , path=\"..\/..\/..\/json-1.0.93\" }  /" 	\
	|sed -e "s/serde = \"1\"/serde={version=\">=1\" , path=\"..\/..\/..\/serde-1.0.156\/serde\"  } /" 	\
	>$F

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
mv $F ${F}.0	# downgrading  ring, sct		# removing feature  "alloc"
cat ${F}.0 \
 	|sed -e "s/\"0.16.20\"/{ version=\"0.16.20\", path=\"..\/..\/ring-0.16.20\" } /" 	\
	|sed -e "s/\"0.7.0\"/{ version=\">=0.6.0\" , path=\"..\/..\/sct.rs-v-0.7.0\" }  /"		\
	|sed -e "s/webpki = { version = \"0.22.0\"/webpki = { version = \"^0.22.0\" , path=\"..\/..\/webpki-0.22.0\" /"		\
	>$F


F=target/ureq-2.6.2/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/rustls = { version = \"/rustls = \{path=\"..\/rustls-v-0.20.8\/rustls\", version=\">=/"		\
		|sed -e "s/url = \"2\"/url=\{version=\">=2.0\", path=\"..\/rust-url-2.3.0\/url\"\} /"	\
		|sed -e "s/webpki = { version = \"0.22\"/webpki=\{version=\">=0.21\" , path=\"..\/webpki-0.22.0\"  /"	\
		|sed -e "s/webpki-roots = {/\webpki-roots={path=\"..\/webpki-roots-v-0.22.6\"\,  /"	\
	>$F

F=target/resvg-0.29.0/usvg/Cargo.toml
mv $F ${F}.0		# downgrading base64 ,  data-url  imagesize   kurbo		rctree	strict-num
cat ${F}.0 	|sed -e "s/\"0.21\"/\">=0.13\" /"\
		|sed -e "s/\"0.2\"/\">=0.1\" /"	\
		|sed -e "s/\"0.11\"/\{version=\"0.11\", path=\"..\/..\/imagesize-0.11.0\"\}  /" \
		|sed -e "s/\"0.9\"/\">=0.7\" /"	\
		|sed -e "s/\"0.5\"/{version=\">=0.5\", path=\"..\/..\/rctree-0.5.0\"\}  /"	\
		|sed -e "s/strict-num = \"0.1\"/strict-num={version=\">=0.1\", path=\"..\/..\/strict-num-0.1.0\"\}  /" \
		>$F

F=target/resvg-0.29.0/rosvgtree/Cargo.toml
mv $F ${F}.0
cat ${F}.0  	|sed -e "s/\"0.18\"/{version=\"=0.18\", path=\"..\/..\/roxmltree-0.18.0\"\}  /" \
	|sed -e "s/\"0.10\"/{version=\"^0.10\", path=\"..\/..\/svgtypes-0.10.0\"\}  /"  	>$F

F=target/roxmltree-0.18.0/Cargo.toml
mv $F ${F}.0		# downgrading xmlparser
cat ${F}.0  |sed -e "s/\"0.13.5\"/{version=\"^0.13.5\", path=\"..\/xmlparser-0.13.5\"\} /" 	>$F


F=target/strict-num-0.1.0/Cargo.toml
mv $F ${F}.0		#  downgrade of  float-cmp,   removing feature  "std"
cat ${F}.0 |sed -e "s/\"0.9\"/\">=0.6\", path=\"..\/float-cmp-0.6.0\" /" |sed -e "s/\"std\"//"	>$F

F=target/webpki-roots-v-0.22.6/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"0.22.0\"/{ version=\">=0.21.0\", path=\"..\/webpki-0.22.0\" } /"	>$F


F=target/webpki-0.22.0/Cargo.toml
mv $F ${F}.0	# downgrade  ring   untrusted
cat ${F}.0  \
	|sed -e "s/\"0.7.1\"/\">=0.7.0\"\n path=\"..\/untrusted-0.7.1\"  /" \
	|sed -e "s/\"0.16.19\"/\"0.16.20\"\npath=\"..\/ring-0.16.20\"  /" 	>$F
#	|sed -e "s/\"0.7.1\"/\"0.7.0\" /" \


## Special case: remove  version number from dependencies.servo_arc
F=target/selectors-0.23.0/Cargo.toml
cat $F |grep "servo_arc" -B100 >${F}.0
cat $F |grep "servo_arc" -A100 |tail -n7 >>${F}.0
# mv $F ${F}.0		#  setting path to cssparser
cat ${F}.0 |sed -e "s/\"0.27\"/\{version=\">=0.27\" , path=\"..\/..\/..\/rust-cssparser-0.28.0\"  \}/"	\
 	|sed -e "s/\"0.99\"/\">=0.99\"\n path=\"..\/derive_more-0.99.17\"  /"	\
	|sed -e "s/\"0.28\"/\">=0.28\"\n path=\"..\/rust-cssparser-0.28.0\" /" \
	|sed -e "s/servo_arc]/servo_arc] \n version=\">=0.1\" \n\n /" \
	|sed -e "s/\"0.1.0\"/\">=0.1.0\" /"	\
	>$F



F=target/convert_case-0.6.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/\"1.9.0\"/\">=1.6.0\" /"	>$F

F=target/derive_more-0.99.17/Cargo.toml
mv $F ${F}.0
cat ${F}.0 |sed -e "s/convert_case = { version = \"0.4\"/convert_case=\{version=\">=0.4\", path=\"..\/convert_case-0.6.0\"\   /"	>$F

F=target/hard-xml-v1.19.0/hard-xml/Cargo.toml
mv $F ${F}.0		#  restoring the version number  here!    env_logger xmlparser
cat ${F}.0 |sed -e "s/\"0.5\"/\{version=\">=0.5\" , path=\"..\/..\/jetscii-0.5.3\"  \}/" \
	|sed -e "s/\"0.0.0\"/\"1.19.0\"/" \
	|sed -e "s/\"0.8\"/\">=0.8\" /" \
    |sed -e "s/\"0.13\"/{version=\">=0.13\", path=\"..\/..\/xmlparser-0.13.5\"\} /" 	\
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
	|sed -e "s/\"1.0.145\"/\">=1.0.145\" \npath=\"..\/..\/serde-1.0.156\/serde\" / " \
	|sed -e "s/\"1.0.37\"/\">=1.0.20\"/" \
	>$F

#  	|sed -e "s/serde = { version = \"1.0.100\"/serde={version=\">=1\" , path=\"..\/serde-1.0.156\/serde\"   /" 	\


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
cat ${F}.0 |sed -e "s/\"0.15\"/\"^0.14.0\"/g" \
 	|sed -e "s/= \"5\"/=  \">=3\"/"  \
	>$F

F=target/rayon-1.5.1/Cargo.toml
mv $F ${F}.0	# downgrade   crossbeam-channel		crossbeam-deque
cat ${F}.0 	 |sed -e "s/\"0.8.0\"/\">=0.7.4\"/"	\
		>$F

F=target/rayon-1.5.1/rayon-core/Cargo.toml
mv $F ${F}.0	# downgrade   crossbeam-channel		crossbeam-deque
cat ${F}.0 |sed -e "s/\"0.5.0\"/\">=0.4\"/" 	\
	 |sed -e "s/\"0.8.0\"/\">=0.7.4\"/"	\
	>$F

F=target/svgtypes-0.10.0/Cargo.toml
mv $F ${F}.0		# downgrade kurbo
cat ${F}.0  	|sed -e "s/\"0.9\"/\">=0.7\"  /" 	>$F


F=target/image-gif-0.12.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/\"0.1.5\"/\{version=\">=0.1.5\" , path=\"..\/lzw-0.1.5\"  \} /"	>$F

F=target/image-tiff-0.8.0/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/\"0.3.0\"/ \"^0.3.0\" , path=\"..\/jpeg-decoder-0.3.0\"  /" \
	|sed -e "s/\"0.1.0\"/\{version=\">=0.1.0\" , path=\"..\/lzw-0.1.5\"  \} /"	>$F

F=target/tracing-tracing-0.1.37/tracing/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/\"0.2.9\"/\">=0.2.7\"   /" 	>$F

F=target/tracing-tracing-0.1.37/tracing-attributes/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/\"1.0.98\"/\">=1.0.76\"   /" 	>$F

F=target/tracing-tracing-0.1.37/tracing-core/Cargo.toml
mv $F ${F}.0
cat ${F}.0  |sed -e "s/\"1.13.0\"/\">=1.9.0\"   /" 	>$F

F=target/sct.rs-v-0.7.0/Cargo.toml
mv $F ${F}.0	# downgrade  ring
cat ${F}.0 	|sed -e "s/\"0.16.20\"/{ version=\"0.16.20\", path=\"..\/ring-0.16.20\" } /" 	>$F

F=target/ring-0.16.20/Cargo.toml
mv $F ${F}.0	# downgrade     untrusted
cat ${F}.0  	|sed -e "s/\"0.7.1\"/\">=0.7.0\"\n path=\"..\/untrusted-0.7.1\" /"	\
 	|sed -e "s/\"0.3.37\"/\">=0.3.37\"\n path=\"..\/web-sys-0.3.37\" /" 	>$F

F=target/hashbrown-0.12.0/Cargo.toml
mv $F ${F}.0	# downgrade ahash
cat ${F}.0 |sed -e "s/\"0.7.0\"/\">=0.7\", path=\"..\/ahash-0.7.6\" /" 		>$F

F=target/fern-fern-0.6.0/Cargo.toml
mv $F ${F}.0	# upgrade colored, widen log, point fern
cat ${F}.0 |sed -e "s/\"1.5\"/\">=1.5\", path=\"..\/colored-1.5.3\" /" 	\
	 |sed -e "s/log = { version = \"0.4\"/log={version=\">=0.4\",  path=\"..\/log-0.4.17\" /" \
	 |sed -e "s/chrono = { version = \"0.4\"/chrono = { version=\"0.4.24\", path=\"..\/chrono-0.4.24\"  /" \
	 |sed -e "s/chrono = \"0.4\"/chrono = { version=\"0.4.24\", path=\"..\/chrono-0.4.24\" } /" \
	 >$F

F=target/colored-1.5.3/Cargo.toml
mv $F ${F}.0	# downgrade lazy_static
cat ${F}.0 |sed -e "s/\"^0.2\"/{version=\"^0.2\", path=\"..\/lazy-static.rs-0.2.11\" } /" 	>$F

F=target/spin-rs-0.9.2/Cargo.toml
mv $F ${F}.0	# point to  lock_api
cat ${F}.0 |sed -e "s/\"0.4\"/\">=0.4\", path=\"..\/parking_lot-lock_api-0.4.9\/lock_api\"  /" 	>$F

F=target/libwebp-sys2-rs-0.1.2/Cargo.toml
mv $F ${F}.0	# point to cfg-if
cat ${F}.0 |sed -e "s/\"0.1.6\"/{ version=\">=0.1.6\", path=\"..\/cfg-if-0.1.7\" } /" 	>$F

F=target/json-1.0.93/Cargo.toml
mv $F ${F}.0	# point to cfg-if	serde
cat ${F}.0 |sed -e "s/itoa = \"1.0\"/itoa={version=\">=1.0\", path=\"..\/itoa-1.0.0\" } /" \
 	|sed -e "s/serde = { version = \"1.0.100\"/serde={version=\">=1\" , path=\"..\/serde-1.0.156\/serde\"   /" 	\
	>$F

F=target/parking_lot-lock_api-0.4.9/lock_api/Cargo.toml
mv $F ${F}.0	#
cat ${F}.0 |sed -e "s/autocfg = \"1.1.0\"/autocfg=\">=1.0\"  /" 	>$F

F=target/chrono-0.4.24/Cargo.toml
mv $F ${F}.0	# upgrade libc, num-traits , iana-time-zone
cat ${F}.0 |sed -e "s/\"0.2.69\"/\">=0.2.69\" , path=\"..\/libc-0.2.103\"/" \
	|sed -e "s/\"0.1.43\"/\">=0.1.43\" , path=\"..\/time-0.1.43\"/" \
	|sed -e "s/\"0.1.45\"/\">=0.1.41\" , path=\"..\/iana-time-zone-0.1.41\"/" \
	|sed -e "s/num-traits = { version = \"0.2\"/num-traits={version=\">=0.2\" , path=\"..\/num-traits-num-traits-0.2.15\"/" \
	|sed -e "s/\"0.2\"/\">=0.2\" , path=\"..\/wasm-bindgen-0.2.81\"  /" \
	|sed -e "s/js-sys = { version = \"0.3\"/js-sys ={version=\">=0.3.58\", path=\"..\/js-sys-0.3.58\" /"	\
	>$F

F=target/clap-3.2.20/Cargo.toml
mv $F ${F}.0	# downgrade strsim , textwrap
cat ${F}.0 |sed -e "s/\"0.10\"/\">=0.9\"/"  	  |sed -e "s/\"0.15.0\"/\">=0.15.0\", path=\"..\/textwrap-0.15.2\"/" 	>$F

F=target/iana-time-zone-0.1.41/Cargo.toml
mv $F ${F}.0	# core-foundation,   wasm-bindgen , android_system_properties
cat ${F}.0 \
	|sed -e "s/\"0.9\"/{version=\">=0.9\", path=\"..\/core-foundation-rs-core-foundation-v0.9.1\/core-foundation\" }/"	\
	|sed -e "s/\"0.3.58\"/{version=\">=0.3.58\", path=\"..\/js-sys-0.3.58\" }/"	\
	|sed -e "s/\"0.2.81\"/{version=\">=0.2.78\"    , path=\"..\/wasm-bindgen-0.2.81\"  }/"	\
	|sed -e "s/\"0.1.4\"/{version=\">=0.1.4\" , path=\"..\/android_system_properties-0.1.4\"  }/"	\
	>$F

F=target/bincode-1.3.3/Cargo.toml
mv $F ${F}.0	# serde
cat ${F}.0	|sed -e "s/\"1.0.63\"/{version=\">=1.0.63\" , path=\"..\/serde-1.0.156\/serde\" }  /" 	>$F

F=target/js-sys-0.3.58/Cargo.toml
mv $F ${F}.0
cat ${F}.0	|sed -e "s/\"0.2.81\"/\"0.2.81\" \npath=\"..\/wasm-bindgen-0.2.81\"  /" 	>$F


F=target/android_system_properties-0.1.4/Cargo.toml
mv $F ${F}.0
cat ${F}.0	|sed -e "s/\"0.2.126\"/\">=0.2.103\" \npath=\"..\/libc-0.2.103\"  /" 	>$F

F=target/serde-1.0.156/serde_derive/Cargo.toml
mv $F ${F}.0	# syn
cat ${F}.0	 |sed -e "s/\"1.0.104\"/{version=\">=1.0.104\", path=\"..\/..\/syn-1.0.109\"} /" \
	>$F

F=target/syn-1.0.109/Cargo.toml
mv $F ${F}.0	# proc-macro2
cat ${F}.0	 |sed -e "s/\"1.0.46\"/\">=1.0.28\" /" \
	|sed -e "s/unicode-ident = \"1.0\"/unicode-ident={version=\">=1\", path=\"..\/unicode-ident-1.0.6\"} /" \
	>$F



F=target/web-sys-0.3.37/Cargo.toml
mv $F ${F}.0
cat ${F}.0	|sed -e "s/\"0.2.60\"/\"0.2.81\" \npath=\"..\/wasm-bindgen-0.2.81\"  /" \
	|sed -e "s/\"0.3.37\"/\"0.3.58\" \npath=\"..\/js-sys-0.3.58\" /"	\
	>$F
##  TODO doppelte Ersetzung !! darf nur bei js-sys wirken





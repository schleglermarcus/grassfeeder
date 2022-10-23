use chrono::DateTime;
use chrono::Local;
use chrono::TimeZone;
use chrono::Utc;
use image::ImageFormat;
use std::io::Cursor;
use std::io::Read;
use textcode::iso8859_1;

static DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn file_exists(filename: &String) -> bool {
    if let Ok(metadata) = std::fs::metadata(&filename) {
        return metadata.is_file();
    }
    false
}

//  escape ampersand as &amp;
pub fn string_escape_url(unescaped: String) -> String {
    let mut s = unescaped.replace('&', "&amp;");
    s = s.replace('<', "&lt;");
    s = s.replace('>', "&gt;");
    s
}

pub fn string_is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

pub fn db_time_to_display(db_time: i64) -> String {
    let fetchd_loc = chrono::offset::Local.timestamp(db_time, 0);
    fetchd_loc.format(DATETIME_FORMAT).to_string()
}

pub fn db_time_to_display_nonnull(db_time: i64) -> String {
    if db_time == 0 {
        return String::default();
    }
    db_time_to_display(db_time)
}

/// Returns the number of non-leap seconds since January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
pub fn timestamp_now() -> i64 {
    let local: DateTime<Local> = Local::now();
    local.timestamp()
}

pub fn timestamp_from_utc(in_utc: DateTime<Utc>) -> i64 {
    let converted: DateTime<Local> = DateTime::from(in_utc);
    converted.timestamp()
}

pub fn convert_webp_to_png(bytes_webp: &[u8], resize_w_h: Option<u32>) -> Result<Vec<u8>, String> {
    let buffersize = 100000;
    let r = image::load_from_memory_with_format(bytes_webp, ImageFormat::WebP);
    if let Err(e) = r {
        return Err(format!("convert_webp_to_png:1 {:?}", e));
    }
    let mut dynimg = r.unwrap();
    if let Some(width) = resize_w_h {
        dynimg = dynimg.thumbnail(width, width);
    }
    let outbuf: Vec<u8> = Vec::with_capacity(buffersize);
    let mut cursor = Cursor::new(outbuf);
    let rw = image::write_buffer_with_format(
        &mut cursor,
        dynimg.as_bytes(),
        dynimg.width(),
        dynimg.height(),
        dynimg.color(),
        ImageFormat::Png,
    );
    match rw {
        Err(e) => Err(format!("convert_webp_to_png:2 {:?}", e)),
        Ok(_written) => Ok(cursor.get_ref().clone()),
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum IconKind {
    None,
    TooSmall,
    Ico,
    Png,
    Bmp,
    Jpg,
    Gif,
    Svg,
    Webp,
    UnknownType, // all analyses done
}

impl Default for IconKind {
    fn default() -> Self {
        IconKind::None
    }
}

pub fn downscale_image(
    img_bytes: &[u8],
    img_type: &IconKind,
    resize_w_h: u32,
) -> Result<Vec<u8>, String> {
    let buffersize = 1000000;
    let img_fo: ImageFormat = match img_type {
        IconKind::Png => ImageFormat::Png,
        IconKind::Jpg => ImageFormat::Jpeg,
        IconKind::Webp => ImageFormat::WebP,
        IconKind::Ico => ImageFormat::Ico,
        IconKind::Bmp => ImageFormat::Bmp,
        _ => {
            warn!("downscale: unhandled format {:?} ", &img_type);
            ImageFormat::Ico
        }
    };
    let r = image::load_from_memory_with_format(img_bytes, img_fo);
    if let Err(e) = r {
        return Err(format!("downscale_png:1: {:?}", e));
    }
    let mut dynimg = r.unwrap();
    dynimg = dynimg.thumbnail(resize_w_h, resize_w_h);
    let mut colortype: image::ColorType = dynimg.color();
    if !is_small_colortype(&colortype) {
        colortype = image::ColorType::Rgba8;
    }
    let outbuf: Vec<u8> = Vec::with_capacity(buffersize);
    let mut cursor = Cursor::new(outbuf);
    let rw = image::write_buffer_with_format(
        &mut cursor,
        dynimg.as_bytes(),
        dynimg.width(),
        dynimg.height(),
        colortype,
        ImageFormat::Png,
    );
    match rw {
        Err(e) => Err(format!("downscale_png:2 {:?}", e)),
        Ok(_written) => Ok(cursor.get_ref().clone()),
    }
}

#[allow(clippy::match_like_matches_macro)]
pub fn is_small_colortype(ct: &image::ColorType) -> bool {
    match ct {
        image::ColorType::L8
        | image::ColorType::La8
        | image::ColorType::Rgb8
        | image::ColorType::Rgba8 => true,
        _ => false,
    }
}

pub fn string_truncate(mut input: String, length: usize) -> String {
    if input.len() > length {
        let slice = input.as_str();
        let mut nlen = length;
        while !slice.is_char_boundary(nlen) {
            nlen += 1;
        }
        input.truncate(nlen);
    }
    input
}

/// Retrieves a Url into a Binary.   Uses maxsize for maximum stored bytes.
///  returns the fetched buffer, the fetched size
pub fn fetch_http_to_bin(url: String, maxsize: usize) -> (Vec<u8>, usize) {
    let response = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(e) => {
            error!("fetching {} => {:?}", &url, &e);
            return (Vec::default(), 0);
        }
    };
    let mut size = maxsize;
    if let Some(h) = response.header("Content-Length") {
        if let Ok(s) = h.parse() {
            if s < maxsize {
                size = s;
            }
        }
    }
    let mut buffer: Vec<u8> = Vec::with_capacity(size);
    let r = response
        .into_reader()
        .take(size as u64)
        .read_to_end(&mut buffer);
    match r {
        Ok(rsize) => (buffer, rsize),
        Err(e) => {
            error!("fetching {} => {:?}", &url, &e);
            (Vec::default(), 0)
        }
    }
}

/// returns String,   was-truncated
// #[allow(dead_code)]
pub fn filter_by_iso8859_1(input: &str) -> (String, bool) {
    let mut dst: Vec<u8> = Vec::new();
    iso8859_1::encode(input, &mut dst);
    match std::str::from_utf8(&dst) {
        Ok(s) => (s.to_string(), false),
        Err(e) => {
            let mut ni: String = input.to_string();
            let mut split_pos = e.valid_up_to();
            while split_pos > 0 && !ni.as_str().is_char_boundary(split_pos) {
                split_pos -= 1;
            }
            let _ = ni.split_off(split_pos);
            (ni, true)
        }
    }
}

///  https://www.freeformatter.com/html-entities.html
///  https://www.w3schools.com/charsets/ref_html_entities_a.asp
pub fn remove_invalid_chars_from_input(inp: String) -> String {
    let mut ret = inp;
    ret = ret.replace(&['\"', '\n', '\'', '\"'][..], "");
    ret = ret.replace("&#38;", "&");
    ret = ret.replace("&#038;", "&");
    ret = ret.replace("&#039;", "\"");
    ret = ret.replace("&#x27;", "\'");
    ret = ret.replace("&gt;", ">");
    ret = ret.replace("&amp;", " & ");
    ret = ret.replace("&#038;", " & ");
    ret = ret.replace("&#128;", "€");
    ret = ret.replace("&#147;", "›");
    ret = ret.replace("&#148;", "-");
    ret = ret.replace("&#xF6;", "ö");
    ret = ret.replace("&#152;", "\'");
    ret = ret.replace("&#153;", " - ");
    ret = ret.replace("&#156;", " - ");
    ret = ret.replace("&#157;", " Š ");
    ret = ret.replace("&#164;", " ");
    ret = ret.replace("&#166;", " ... ");
    ret = ret.replace("&#190;", "¾");
    ret = ret.replace("&#220;", "Ü");
    ret = ret.replace("&#223;", "ß");
    ret = ret.replace("&#226;", "\"");
    ret = ret.replace("&#228;", "ä");
    ret = ret.replace("&#246;", "ö");
    ret = ret.replace("&#252;", "ü");
    ret = ret.replace("&#x166;", " ... ");
    ret = ret.replace("&#8211;", "\"");
    ret = ret.replace("&#8220;", "\"");
    ret = ret.replace("&#8221;", "\"");
    ret = ret.replace("&#8216;", "\'");
    ret = ret.replace("&#8217;", "\'");
    ret = ret.replace("&#8230;", " ... ");
    ret = ret.replace("&#8242;", "\'");
    ret = ret.replace("&#x8211;", " - ");
    ret = ret.replace("&quot;", "\"");
    ret.trim().to_string()
}

// --- mini state machine

pub trait Step<S> {
    fn step(self: Box<Self>) -> StepResult<S>;
}

pub enum StepResult<I> {
    Continue(Box<dyn Step<I>>),
    Stop(I),
}

impl<I> StepResult<I> {
    fn run(mut self) -> I {
        loop {
            match self {
                StepResult::Continue(stm) => self = stm.step(),
                StepResult::Stop(ii) => return ii,
            }
        }
    }

    pub fn start(first: Box<dyn Step<I>>) -> I {
        StepResult::Continue(first).run()
    }
}

// ---

#[cfg(test)]
mod t {
    use super::*;
    use crate::util::fetch_http_to_bin;
    use crate::util::remove_invalid_chars_from_input;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn image_webp_to_png() {
        let file_in = "../fr_core/tests/data/lupoca.webp";
        let file_out = "../target/lupoca.png";
        let webpdata: Vec<u8> = crate::web::mockfilefetcher::file_to_bin(file_in).unwrap();
        let outdata = convert_webp_to_png(&webpdata, Some(20)).unwrap();
        let mut file = File::create(file_out).unwrap();
        let w_r = file.write_all(&outdata);
        assert!(w_r.is_ok());
        assert!(outdata.len() >= 1151 && outdata.len() <= 1152);
        //         debug!("{} bytes written {:?}", outdata.len(), w_r);
    }

    //cargo watch -s "cargo    test  util::util_fetch_test::sanitize_input   --lib  -- --exact "
    #[test]
    pub fn sanitize_input() {
        assert_eq!(
            remove_invalid_chars_from_input(" h ".to_string()),
            "h".to_string()
        );
        assert_eq!(
            remove_invalid_chars_from_input("a\nb\'c".to_string()),
            "abc".to_string()
        );
        assert_eq!(
            remove_invalid_chars_from_input("&#8220;Science&#8221; no ".to_string()),
            "\"Science\" no".to_string()
        );
        assert_eq!(
            remove_invalid_chars_from_input("adviser&#8217;s".to_string()),
            "adviser's".to_string()
        );
        assert_eq!(
            remove_invalid_chars_from_input("Jenkins &#226;&#128;&#147; Brighteon".to_string()),
            "Jenkins \"€› Brighteon".to_string()
        );
    }

    #[test]
    fn namefilter1() {
        assert_eq!(
            filter_by_iso8859_1(&"Hallo".to_string()),
            ("Hallo".to_string(), false)
        );
        assert_eq!(
            filter_by_iso8859_1(&"news 기사 요약 -".to_string()),
            ("news ".to_string(), true)
        );
        assert_eq!(
            filter_by_iso8859_1(&"Japan 無料ダウンロード".to_string()),
            ("Japan ".to_string(), true)
        );
        assert_eq!(
            filter_by_iso8859_1(&"J 無料ダウ".to_string()),
            ("J ".to_string(), true)
        );
        assert_eq!(
            filter_by_iso8859_1(&" 料ダウ".to_string()),
            (" ".to_string(), true)
        );
    }

    #[test]
    fn string_truncate_reg() {
        let jap = String::from("Japan 無料ダウンロード");
        let short = string_truncate(jap, 10);
        assert_eq!(short, String::from("Japan 無料"));
    }

    #[test]
    fn string_truncate_long() {
        let jap = String::from("Japan 無料ダウンロード");
        let short = string_truncate(jap, 100);
        assert_eq!(short, String::from("Japan 無料ダウンロード"));
    }

    #[test]
    fn fetch_infowars() {
        let (_buf, size) =
            fetch_http_to_bin(String::from("http://www.infowars.com/favicon.ico"), 10000);
        assert_eq!(_buf.len(), size);
        assert!([0, 1150].contains(&size));
    }

    /*
    Transport(Transport { kind: Dns, message: Some("resolve dns name 'www.chip.de:443'"), service not known" }) })
    */
    #[test]
    fn fetch_chip() {
        let (_buf, size) = fetch_http_to_bin(
            String::from("https://www.chip.de/fec/assets/favicon/favicon-32x32.png?v=01"),
            1000,
        );
        assert_eq!(size, 694);

        let (_buf, size) = fetch_http_to_bin(
            String::from("https://www.chip.de/fec/assets/favicon/favicon-32x32.png?v=01"),
            100,
        );
        assert_eq!(size, 100);
        let (buf, size) = fetch_http_to_bin(
            String::from("https://www.chip.de/fec/assets/favicon/favicon-32x32.404"),
            10000,
        );
        assert_eq!(size, 0);
        assert_eq!(buf.len(), size);
    }

    // ---

    struct State1(Inner);
    impl Step<Inner> for State1 {
        fn step(self: Box<Self>) -> StepResult<Inner> {
            let mut inn: Inner = self.0;
            inn.i += 2;
            StepResult::Continue(Box::new(Exit(inn)))
        }
    }

    struct Exit(Inner);
    impl Step<Inner> for Exit {
        fn step(self: Box<Self>) -> StepResult<Inner> {
            StepResult::Stop(self.0)
        }
    }

    struct Inner {
        i: u16,
    }

    #[test]
    fn mini_with_tuple() {
        let last_data = StepResult::start(Box::new(State1(Inner { i: 3 })));
        assert_eq!(last_data.i, 5);
    }
}

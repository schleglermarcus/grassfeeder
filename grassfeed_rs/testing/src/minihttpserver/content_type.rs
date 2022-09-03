#![allow(dead_code)]

#[derive(PartialEq, Debug, Eq)]
pub enum ContentType {
    CSS,
    GIF,
    HTML,
    JPEG,
    PNG,
    SVG,
    TEXT,
    XML,
    RSS,
    ICO,
    UNKNOWN,
}

impl ContentType {
    pub fn from_file_ext(ext: &str) -> ContentType {
        match ext {
            "css" => ContentType::CSS,
            "gif" => ContentType::GIF,
            "htm" => ContentType::HTML,
            "html" => ContentType::HTML,
            "jpeg" => ContentType::JPEG,
            "jpg" => ContentType::JPEG,
            "png" => ContentType::PNG,
            "svg" => ContentType::SVG,
            "txt" => ContentType::TEXT,
            "xml" => ContentType::XML,
            "rss" => ContentType::RSS,
            "ico" => ContentType::ICO,
            _ => ContentType::UNKNOWN,
        }
    }
    pub fn value(&self) -> &str {
        match *self {
            ContentType::CSS => "text/css",
            ContentType::GIF => "image/gif",
            ContentType::HTML => "text/html",
            ContentType::JPEG => "image/jpeg",
            ContentType::PNG => "image/png",
            ContentType::SVG => "image/svg+xml",
            ContentType::TEXT => "text/plain",
            ContentType::XML => "application/xml",
            ContentType::RSS => "application/rss+xml",
            ContentType::ICO => "application/x-icon",
            ContentType::UNKNOWN => "application/octet-stream",
            //  _ => "application/octet-stream",
        }
    }
}

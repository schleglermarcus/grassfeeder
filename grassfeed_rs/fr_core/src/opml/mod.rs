pub mod opmlreader;

#[cfg(test)]
mod tests {

    use opml::Outline;

    #[allow(dead_code)]
    fn build_with_outlines() {
        let mut group1 = Outline::default();
        group1.title = Some("Group1".to_string());
        group1.text = "group-1".to_string();
        let mut feed2 = Outline::default();
        feed2.xml_url = Some("http://2/some.xml".to_string());
        feed2.text = "feed-2".to_string();
        group1.outlines.push(feed2);

        let mut feed3 = Outline::default();
        feed3.xml_url = Some("http://3/some.xml".to_string());
        feed3.text = "feed3".to_string();
        group1.outlines.push(feed3);
    }
}

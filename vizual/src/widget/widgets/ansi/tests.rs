use super::Content;

#[test]
fn append_keeps_open_hyperlinks_across_sequences() {
    let mut content = Content::new("before \u{1b}]8;;https://example.com\u{1b}\\");
    content.append("link");
    content.append("\u{1b}]8;;\u{1b}\\ after");

    assert_eq!(content.text().content, "before link after");
    assert_eq!(
        content.text().hyperlinks().collect::<Vec<_>>(),
        vec![crate::graphics::text::Hyperlink {
            range: 7..11,
            url: "https://example.com".into(),
        }]
    );
}

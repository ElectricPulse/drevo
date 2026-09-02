use super::Styled_text;

#[test]
fn parses_osc8_hyperlinks_terminated_by_string_terminator() {
    let text = Styled_text::ansi(
        "before \u{1b}]8;id=docs;https://example.com/docs\u{1b}\\\u{1b}[31mDocs\u{1b}[0m\u{1b}]8;;\u{1b}\\ after",
    );

    assert_eq!(text.content, "before Docs after");
    assert_eq!(
        text.hyperlinks().collect::<Vec<_>>(),
        vec![super::Hyperlink {
            range: 7..11,
            url: "https://example.com/docs".into(),
        }]
    );
}

#[test]
fn parses_osc8_hyperlinks_terminated_by_bell() {
    let text = Styled_text::ansi("\u{1b}]8;;https://example.com/a;b\u{7}link\u{1b}]8;;\u{7} plain");

    assert_eq!(text.content, "link plain");
    assert_eq!(
        text.hyperlinks().collect::<Vec<_>>(),
        vec![super::Hyperlink {
            range: 0..4,
            url: "https://example.com/a;b".into(),
        }]
    );
}

#[test]
fn parses_osc8_hyperlinks_using_c1_control_codes() {
    let text = Styled_text::ansi("\u{9d}8;;https://example.com\u{9c}link\u{9d}8;;\u{9c}");

    assert_eq!(text.content, "link");
    assert_eq!(
        text.hyperlinks().collect::<Vec<_>>(),
        vec![super::Hyperlink {
            range: 0..4,
            url: "https://example.com".into(),
        }]
    );
}

#[test]
fn appending_ansi_keeps_the_active_sgr_style() {
    let mut text = Styled_text::empty();
    let mut parser = super::Ansi_parser::default();
    text.append_ansi("\u{1b}[31m", &mut parser);
    text.append_ansi("red", &mut parser);

    assert_eq!(text.content, "red");
    assert_eq!(text.spans.len(), 1);
    assert_eq!(
        text.spans[0].style.foreground,
        crate::style::Color::Indexed(1)
    );
}

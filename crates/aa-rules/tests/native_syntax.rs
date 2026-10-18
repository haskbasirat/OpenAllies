use aa_rules::tgi::{Document, Item, Limits, SyntaxNoticeKind};

#[test]
fn native_physical_header_boundary_preserves_source() {
    let bytes = b"[Thing Template=Base\r\n{\r\n IDS = sample\r\n}\r\n";
    assert!(Document::parse(bytes.to_vec()).is_err());
    let d = Document::parse_native(bytes.to_vec(), Limits::default()).unwrap();
    assert_eq!(d.original_bytes(), bytes);
    assert_eq!(d.notices.len(), 1);
    assert_eq!(d.notices[0].kind, SyntaxNoticeKind::HeaderEndedAtLineEnd);
    assert_eq!(
        d.text(d.nodes()[0].header.span),
        Some("[Thing Template=Base")
    );
}

#[test]
fn native_eof_closes_plain_nested_bodies_without_synthetic_bytes() {
    let bytes = b"[Thing]{\n IDS = sample\n [Body]{\n health = 200\n";
    let d = Document::parse_native(bytes.to_vec(), Limits::default()).unwrap();
    assert_eq!(d.original_bytes(), bytes);
    assert_eq!(d.notices.len(), 2);
    assert!(
        d.notices
            .iter()
            .all(|n| n.kind == SyntaxNoticeKind::BodyEndedAtEof)
    );
    let Item::Node(child) = &d.nodes()[0].items[1] else {
        panic!()
    };
    assert_eq!(d.header_name(&child.header), Some("Body"));
}

#[test]
fn native_mode_rejects_unterminated_quotes_and_extra_closures() {
    for bytes in ["[Thing]{\n name = \"unfinished", "[Thing]{} }"] {
        assert!(Document::parse_native(bytes.as_bytes().to_vec(), Limits::default()).is_err());
    }
    // A delayed body is native (5BD080 oracle in native_tgi_parser.rs).
    let d = Document::parse_native(
        b"[Thing]{\n [Size]\n scale = 1\n {\n radius=2\n}\n}".to_vec(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(d.notices[0].kind, SyntaxNoticeKind::DelayedBody);
}

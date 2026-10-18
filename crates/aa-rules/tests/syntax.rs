use aa_rules::tgi::{Document, Item, Limits};

#[test]
fn duplicate_identifiers_and_unevaluated_expression_are_retained() {
    let d = Document::parse(b"[Template One Template=Base]\n{\n IDS = same\n value = $other == 4 ? A : B\n value = 7\n}\n[Thing Inherit=One]\nIDS = same\n".to_vec()).unwrap();
    let nodes = d.nodes();
    assert_eq!(nodes.len(), 2);
    assert_eq!(
        d.text(nodes[0].header.span),
        Some("[Template One Template=Base]")
    );
    assert_eq!(d.text(nodes[1].header.span), Some("[Thing Inherit=One]"));
    let Item::Property(first) = &nodes[0].items[1] else {
        panic!()
    };
    let Item::Property(second) = &nodes[0].items[2] else {
        panic!()
    };
    assert_eq!(d.property_name(first), d.property_name(second));
    assert_eq!(d.property_value(first), Some("$other == 4 ? A : B"));
    assert_eq!(d.property_value(second), Some("7"));
}
#[test]
fn quotes_comments_expressions_and_original_bytes() {
    let bytes=b"\xef\xbb\xbf/* [fake] { } */\r\n[Template Keep template=Base]\r\n{\r\n value = (other * 2) + \"a;[b]//x\" ; ignored\r\n url = http://example/a\r\n string Declared\r\n empty =\r\n}\r\n";
    let d = Document::parse(bytes.to_vec()).unwrap();
    assert_eq!(d.original_bytes(), bytes);
    let n = d.nodes()[0];
    assert!(n.braced);
    assert_eq!(d.header_name(&n.header), Some("Template"));
    let props: Vec<_> = n
        .items
        .iter()
        .map(|i| match i {
            Item::Property(p) => p,
            _ => panic!(),
        })
        .collect();
    assert_eq!(d.property_name(props[0]), Some("value"));
    assert_eq!(
        d.property_value(props[0]),
        Some("(other * 2) + \"a;[b]//x\"")
    );
    assert_eq!(d.property_value(props[1]), Some("http://example/a"));
    assert!(props[2].value.is_none());
    assert_eq!(d.property_value(props[3]), Some(""));
}
#[test]
fn unbraced_children_keep_order_and_case() {
    let d = Document::parse(
        b"[Root] {\n [Child a]\n IDS = first\n [Child b]\n IDS = second\n}\n".to_vec(),
    )
    .unwrap();
    let nodes = d.nodes();
    assert_eq!(nodes.len(), 3);
    assert!(nodes[0].braced);
    assert!(!nodes[1].braced);
    assert!(!nodes[2].braced);
    for (n, value) in nodes[1..].iter().zip(["first", "second"]) {
        let Item::Property(p) = &n.items[0] else {
            panic!()
        };
        assert_eq!(d.property_name(p), Some("IDS"));
        assert_eq!(d.property_value(p), Some(value));
    }
}
#[test]
fn legacy_bytes_are_not_lossily_decoded() {
    let raw = b";copyright \xa9\r\n[Text] {\n label = \"caf\xe9\"\n}\n";
    let d = Document::parse(raw.to_vec()).unwrap();
    assert_eq!(d.original_bytes(), raw);
    let Item::Property(p) = &d.nodes()[0].items[0] else {
        panic!()
    };
    assert_eq!(d.property_value(p), None);
    assert_eq!(d.bytes(p.value.unwrap()).unwrap(), b"\"caf\xe9\"");
}
#[test]
fn malformed_and_limited_sources_fail_closed() {
    for raw in [
        "[a] {",
        "[a",
        "[]",
        "[a] { } }",
        "[a] { x = \"bad }",
        "/* open",
        "{x=1}",
        "x\0y",
    ] {
        assert!(Document::parse(raw.as_bytes().to_vec()).is_err(), "{raw}");
    }
    assert!(
        Document::parse_with_limits(
            b"[a]{[b]{[c]{}}}".to_vec(),
            Limits {
                max_depth: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        Document::parse_with_limits(
            b"[a]".to_vec(),
            Limits {
                max_tokens: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        Document::parse_with_limits(
            b"abc".to_vec(),
            Limits {
                max_bytes: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn overlay_ledger_is_fixed() {
    let rows = aa_rules::sources::v14_overlay_identities().unwrap();
    assert_eq!(rows.len(), 23);
    assert_eq!(rows.iter().filter(|r| r.path.ends_with(".tgi")).count(), 20);
    assert!(
        !rows
            .iter()
            .any(|r| r.path.contains("Profile") || r.path.contains("hotkeys_custom"))
    );
}

#[test]
fn native_key_scanner_keeps_brackets_inside_property_identifiers() {
    let raw = b"[Text]\r\n{\r\n key_[ = \"[\"\r\n key_] = \"]\"\r\n name[variant] = null\r\n}\r\n";
    let d = Document::parse(raw.to_vec()).unwrap();
    let n = d.nodes()[0];
    let names: Vec<_> = n
        .items
        .iter()
        .map(|i| {
            let Item::Property(p) = i else { panic!() };
            d.property_name(p).unwrap()
        })
        .collect();
    assert_eq!(names, ["key_[", "key_]", "name[variant]"]);
    assert_eq!(d.original_bytes(), raw);
}

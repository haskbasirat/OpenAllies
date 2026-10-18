use aa_rules::{
    sources::{
        Origin, SourceLimits, V14_NATIVE_SHA256, V14Sources, sha256, v14_overlay_identities,
    },
    tgi::{Document, Item, Node},
};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn runtime() -> PathBuf {
    root().join("../reconstruction/native-game/runtime-expanded-v14")
}
fn test_budget() -> SourceLimits {
    SourceLimits {
        max_member_bytes: 64 * 1024 * 1024,
    }
}
fn sources() -> &'static Mutex<V14Sources> {
    static S: OnceLock<Mutex<V14Sources>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(V14Sources::open(root().join("../Data.rwd"), runtime(), test_budget()).unwrap())
    })
}
fn parse(s: &mut V14Sources, p: &str) -> Document {
    let r = s.read(p).unwrap();
    Document::parse(r.bytes).unwrap()
}
fn props<'a>(d: &'a Document, name: &str) -> Vec<&'a str> {
    fn visit<'a>(d: &'a Document, items: &'a [Item], name: &str, out: &mut Vec<&'a str>) {
        for i in items {
            match i {
                Item::Node(n) => visit(d, &n.items, name, out),
                Item::Property(p) => {
                    if d.property_name(p) == Some(name) {
                        out.push(d.property_value(p).unwrap());
                    }
                }
            }
        }
    }
    let mut result = Vec::new();
    visit(d, &d.items, name, &mut result);
    result
}
fn node_prop<'a>(d: &'a Document, n: &'a Node, name: &str) -> Option<&'a str> {
    n.items.iter().find_map(|i| match i {
        Item::Property(p) if d.property_name(p) == Some(name) => d.property_value(p),
        _ => None,
    })
}
#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn exact_v14_rules_and_stock_fallback() {
    let mut s = sources().lock().unwrap();
    for p in [
        "../Profile/data/User/UVars.tgi",
        "/Game/SVars.tgi",
        "C:/secret",
        "Game//SVars.tgi",
    ] {
        assert!(s.read(p).is_err());
    }
    let replacement = s.read("gAmE\\sVaRs_Aa.TgI").unwrap();
    assert_eq!(replacement.origin, Origin::V14Loose);
    assert_eq!(replacement.logical_path, "Game/SVars_AA.tgi");
    let stock = s.read_original("Game/SVars_AA.tgi").unwrap();
    assert_ne!(replacement.sha256, stock.sha256);
    let stock = Document::parse(stock.bytes).unwrap();
    assert_eq!(props(&stock, "EconomyLimitedResourceMax"), ["20"]);
    assert_eq!(props(&stock, "KingdomsMax"), ["8"]);
    let d = Document::parse(replacement.bytes).unwrap();
    assert_eq!(props(&d, "EconomyLimitedResourceMax"), ["100"]);
    assert_eq!(props(&d, "KingdomsMax"), ["16"]);
    let tick = s.read("Game/SVars.tgi").unwrap();
    assert!(matches!(tick.origin, Origin::OriginalArchive { .. }));
    assert_eq!(
        props(
            &Document::parse(tick.bytes).unwrap(),
            "GameTicksPerGameSecond"
        ),
        ["16"]
    );
    assert_eq!(
        props(&parse(&mut s, "AVars.tgi"), "MultiplayerMaxNumPlayers"),
        ["16"]
    );
    assert_eq!(
        props(
            &parse(&mut s, "Special/Things/thing_start_location.tgi"),
            "building_limit_provided"
        ),
        ["100"]
    );
    let d = parse(&mut s, "Game/connection_types.tgi");
    let rows: Vec<_> = d
        .nodes()
        .iter()
        .map(|n| {
            (
                node_prop(&d, n, "ids").unwrap(),
                node_prop(&d, n, "maxplayers").unwrap(),
            )
        })
        .collect();
    assert_eq!(rows, [("Modem", "4"), ("Broadband", "16")]);
    let d = parse(&mut s, "Templates/template_rmc_aa.tgi");
    let maps: Vec<_> = d
        .nodes()
        .into_iter()
        .filter(|n| d.header_name(&n.header) == Some("MapSize"))
        .map(|n| {
            (
                node_prop(&d, n, "width").unwrap(),
                node_prop(&d, n, "height").unwrap(),
            )
        })
        .collect();
    assert_eq!(
        maps,
        [
            ("320", "320"),
            ("384", "384"),
            ("448", "448"),
            ("512", "512"),
            ("576", "576"),
            ("640", "640"),
            ("768", "768"),
            ("1024", "1024")
        ]
    );
    for index in 9..=16 {
        let id = format!("side{index:02}");
        let matching: Vec<_> = d
            .nodes()
            .into_iter()
            .filter(|n| {
                d.header_name(&n.header) == Some("Kingdom")
                    && node_prop(&d, n, "IDS") == Some(id.as_str())
            })
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(node_prop(&d, matching[0], "playable"), Some("true"));
    }
    let catalog = s.catalog();
    assert_eq!(
        catalog
            .iter()
            .filter(|r| r.origin == Origin::V14Loose)
            .count(),
        23
    );
    assert!(s.read_original("AVars_ClientCadence.tgi").is_err());
    assert_eq!(
        s.read("AVars_ClientCadence.tgi").unwrap().origin,
        Origin::V14Loose
    );
    // Zoom is compiled native data, NOT a TGI evaluation result.
    let native = std::fs::read(runtime().join("AA.exe")).unwrap();
    assert_eq!(sha256(&native), V14_NATIVE_SHA256);
    assert_eq!(&native[9287..9291], &2.1f32.to_le_bytes());
}
#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn actual_corpus_preserves_bytes_and_reports_unsupported_syntax() {
    let mut s = sources().lock().unwrap();
    let mut errors = Vec::new();
    let mut parsed = 0;
    for row in s
        .catalog()
        .into_iter()
        .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
    {
        let r = s.read(&row.logical_path).unwrap();
        match Document::parse(r.bytes.clone()) {
            Ok(d) => {
                assert_eq!(d.original_bytes(), r.bytes);
                parsed += 1;
            }
            Err(_) => errors.push(row.logical_path),
        }
    }
    errors.sort();
    let mut expected: Vec<String> =
        serde_json::from_str(include_str!("fixtures/phase2-unsupported-tgi-paths.json")).unwrap();
    expected.sort();
    assert_eq!(errors, expected);
    assert_eq!(parsed, 1835);
}
#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn corrupted_or_missing_overlay_cannot_fall_back_to_stock() {
    let unique = format!(
        "aa-rules-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let temp = std::env::temp_dir().join(unique);
    std::fs::create_dir(&temp).unwrap();
    for row in v14_overlay_identities().unwrap() {
        let target = temp.join(&row.path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(runtime().join(&row.path), target).unwrap();
    }
    let path = temp.join("Data/Game/SVars_AA.tgi");
    let mut modified = std::fs::read(&path).unwrap();
    modified[0] ^= 1;
    std::fs::write(&path, modified).unwrap();
    let archive = root().join("../Data.rwd");
    assert!(matches!(
        V14Sources::open(&archive, &temp, test_budget()),
        Err(aa_rules::sources::SourceError::Integrity(_))
    ));
    std::fs::remove_file(&path).unwrap();
    assert!(V14Sources::open(&archive, &temp, test_budget()).is_err());
    assert!(matches!(
        V14Sources::open(
            &archive,
            runtime(),
            SourceLimits {
                max_member_bytes: 1
            }
        ),
        Err(aa_rules::sources::SourceError::TooLarge(_))
    ));
    // Remove only this test's unique temporary directory, after containment check.
    assert!(temp.starts_with(std::env::temp_dir()));
    assert!(
        temp.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("aa-rules-")
    );
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn actual_v14_ordered_resource_catalog() {
    let mut s = sources().lock().unwrap();
    let c = aa_rules::resources::ResourceCatalog::load_v14(&mut s).unwrap();
    assert_eq!(c.resources.len(), 7);
    assert_eq!(c.sources.len(), 6);
    assert_eq!(c.stockpiled_resource_max.raw, "99999");
    assert_eq!(c.limited_resource_max, 100);
    for source in &c.sources {
        let actual = s.read(&source.path).unwrap();
        assert_eq!(source.sha256, actual.sha256);
        assert_eq!(source.origin, actual.origin);
    }
    assert_eq!(c.find("ammo").unwrap().declaration_ordinal, 1);
    assert_eq!(c.find("ammo").unwrap().resource_list_ordinal, 2);
    assert_eq!(
        c.find("building_limit_provided").unwrap().purchase_cost[0]
            .value
            .f32_bits,
        0
    );
}

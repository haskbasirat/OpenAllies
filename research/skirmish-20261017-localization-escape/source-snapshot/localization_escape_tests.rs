use super::*;
#[test]
fn original_display_escapes_keep_order_and_diagnostic_prefix() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-localization-escape/native-display-escape.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let i = &row["input"];
        let mut text = i["text"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect::<Vec<_>>();
        let mut lookups = Vec::new();
        let result = decode(
            &mut text,
            i["start"].as_u64().unwrap() as u16,
            i["end"].as_u64().unwrap() as u32,
            i["single"].as_u64().unwrap() as u16,
            !i["glyphs"].as_object().unwrap().is_empty(),
            &mut |name| {
                let name = String::from_utf16(name).unwrap();
                lookups.push(name.clone());
                Ok(i["glyphs"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&name))
                    .map(|(_, v)| v.as_u64().unwrap() as u16))
            },
        );
        assert_eq!(
            result.is_ok(),
            row["native"]["outcome"] == "complete",
            "{row}"
        );
        assert_eq!(serde_json::json!(text), row["native"]["text"], "{row}");
        assert_eq!(
            serde_json::json!(lookups),
            row["native"]["lookups"],
            "{row}"
        );
    }
}

#[test]
#[ignore = "requires actual source files; source census only, no World readiness"]
fn actual_font_projection_uses_canonical_glyph_manager_all_pc() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let table = Table::load(&mut sources, NativeOverrides::ControlledNone).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut bound = Bound {
            table: &table,
            actors: &actors,
            pc,
        };
        for (id, projection) in &table.glyphs {
            assert_eq!(
                bound.find(&id.encode_utf16().collect::<Vec<_>>()).unwrap(),
                Some(projection.index)
            );
            rows.push(serde_json::json!({"pc":pc,"ids":id,"index":projection.index,"source":projection.source}));
        }
        let text = "&ch_faction_experience.Hedges";
        let mut text = text.encode_utf16().collect::<Vec<_>>();
        table.decode_bound(&actors, pc, &mut text).unwrap();
        assert_eq!(text[1..], "Hedges".encode_utf16().collect::<Vec<_>>());
    }
    let snapshot = serde_json::json!({"scope":"Fresh immutable source projection/canonical actual Glyph membership and typed index census; no completed World or rendered font claim","files":table.files,"variables":table.variables,"start":table.start,"end":table.end,"single":table.single,"rows":rows});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-localization-escape/actual-source-census.json");
    if let Some(output) = std::env::var_os("AA_CAPTURE_LOCALIZATION_ESCAPE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)
                .unwrap(),
            &snapshot,
        )
        .unwrap();
    } else {
        assert_eq!(
            snapshot,
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()).unwrap()
        );
    }
}

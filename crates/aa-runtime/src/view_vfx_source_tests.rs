use super::*;
#[test]
fn real_vfx_constructor_virtual_and_lookup_matrix() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-view-asset-backend/native-vfx-lookup.json"
    ))
    .unwrap();
    for row in f["rows"].as_array().unwrap() {
        for t in row["native"]["types"].as_array().unwrap() {
            assert_eq!(
                vfx_virtual_48(t["kind"].as_u64().unwrap() as u32).unwrap(),
                t["value"] == 1
            );
            assert_eq!(t["descriptor_kind"], t["kind"]);
        }
        let q = row["native"]["queries"].as_array().unwrap();
        assert_eq!(
            q.iter().find(|v| v["query"] == "old").unwrap()["vfx"],
            "kind12"
        );
        assert!(q.iter().find(|v| v["query"] == "kind12").unwrap()["vfx"].is_null());
    }
    assert!(vfx_virtual_48(4).is_err());
    assert_eq!(name(&[b'x' as u16, 0, 0xd800]).unwrap(), "x");
    assert!(name(&[0xd800]).is_err());
}
#[test]
#[ignore = "requires actual source-owned Catalog/completed Bank and current aliases"]
fn actual_current_vfx_aliases_are_same_registered_descriptor_references() {
    use crate::{
        combat_source_receipt::VerifiedStartupReceipt, load_view_variables::NativeOverrides,
        view_asset_aliases,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let aliases = view_asset_aliases::State::from_fresh_sources(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let actual_source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-view-asset-backend/actual-source-probe.json"
    ))
    .unwrap();
    let mut effects = vec![];
    for key in crate::view_init_assets::alias_keys().filter(|k| !k.starts_with("Texture")) {
        let alias = aliases.alias_410cb0(&catalog, &bank, key).unwrap();
        let expected = actual_source["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == key)
            .unwrap();
        assert_eq!(
            String::from_utf16(alias.units()).unwrap(),
            expected["query"]
        );
        let effect = find(&catalog, &bank, alias.units()).unwrap();
        if expected["actual_find"].is_null() {
            assert_eq!(key, "VFXAttackEfficiencyBar");
            assert!(
                effect.is_none(),
                "actual missing alias {key} must remain NULL"
            );
            continue;
        }
        let effect =
            effect.unwrap_or_else(|| panic!("actual registered alias {key} is unexpectedly NULL"));
        assert_eq!(effect.id(), expected["actual_find"]["id"]);
        assert_eq!(
            effect.kind(&catalog, &bank).unwrap(),
            expected["actual_find"]["kind"]
        );
        assert_eq!(
            serde_json::to_value(effect.sources()).unwrap(),
            expected["actual_find"]["sources"]
        );
        assert!(vfx_virtual_48(effect.kind(&catalog, &bank).unwrap()).unwrap());
        let repeat = find(&catalog, &bank, alias.units()).unwrap().unwrap();
        assert!(effect.same_identity(&repeat));
        if matches!(key, "VFXDamageNumbers" | "VFXHealNumbers") {
            assert_eq!(effect.kind(&catalog, &bank).unwrap(), 12);
        }
        effects.push((key, effect));
    }
    assert_eq!(effects.len(), 17);
    assert!(find(&catalog, &bank, &[]).unwrap().is_none());
    assert!(find(
        &catalog,
        &bank,
        &"definitely_missing_vfx".encode_utf16().collect::<Vec<_>>()
    )
    .unwrap()
    .is_none());
    assert!(find(&catalog, &catalog.definition_bank.borrow(), &[]).is_err());
    println!("18 actual current VFX queries retain17 canonical descriptor identities and the real NULL attack-efficiency alias with same moved Bank; no preload/payload completion");
}

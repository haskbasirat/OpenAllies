//! Loads the shipped metagame definitions from the verified original
//! Data.rwd + accepted v14 overlays (requires the local installation) and
//! checks them against the authored files and the native qsort fixture.
use aa_metagame::battle::DestroyBehavior;
use aa_metagame::data::{ActorSpacing, MetaGameData, TerritoryKind};
use aa_numeric::Precision;
use aa_rules::sources::{SourceLimits, V14Sources};
use serde_json::Value;
use std::path::PathBuf;

fn load() -> MetaGameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let mut sources = V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 << 20,
        },
    )
    .unwrap();
    MetaGameData::load_v14(&mut sources, Precision::Extended64).unwrap()
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn shipped_metagame_definitions_load() {
    let data = load();
    // SVars.tgi: 0.05 and 0.47 as binary32.
    assert_eq!(data.settings.technology_bonus_bits, 0x3d4c_cccd);
    assert_eq!(data.settings.attack_win_probability_bits, 0x3ef0_a3d7);
    println!("unresolved non-metagame things: {}", data.unresolved_things.len());
    for u in &data.unverified_references {
        println!("unverified: {} {} ({})", u.field, u.ids, u.reason);
    }
    // The shared parser now accepts Units/USA/usa_carrier.tgi (aa-rules fix,
    // 2026-10-14), so every Thing reference is verified against the catalog.
    assert!(data.unverified_references.is_empty());

    // Localized fields (5B5620 / 47D960): authored keys, not translated.
    let c0 = &data.campaigns[0];
    assert_eq!(c0.name, "#mg_campaigns_usa_dynamic_campaign_name");
    assert_eq!(c0.description, "#mg_campaigns_usa_dynamic_campaign_description");
    assert_eq!(c0.kingdoms[1].name, "#mg_campaigns_kingdom_jpn_name");
    assert_eq!(data.divisions[0].upgrade_tooltip, "#mg_division_MGDivisionInfantry_upgrade_tooltip");
    assert_eq!(data.commands[0].tooltip, "#mg_command_move_tooltip");
    assert_eq!(data.ai[0].name, "#mg_ai_usa_worldmap_mgai_name");

    // MGMap
    assert_eq!(data.maps.len(), 1);
    let map = &data.maps[0];
    assert_eq!(map.ids, "worldmap");
    assert_eq!(map.name, "#mg_map_MGMapTemplate_name");
    assert_eq!(map.width_bits, 144f32.to_bits());
    assert_eq!(map.height_bits, 72f32.to_bits());
    assert_eq!(map.minimap_blend_amount_bits, 0.4f32.to_bits());
    assert_eq!(map.territories.len(), 116);
    let gib = &map.territories[0];
    assert_eq!(gib.ids, "Gibraltar");
    assert_eq!(gib.kind, TerritoryKind::Normal);
    assert_eq!(gib.object_ids, "mgmap_Gibraltar", "map instance overrides template object_ids");
    assert_eq!(gib.production, 2);
    assert_eq!(gib.minimap_alpha, 53);
    assert_eq!(gib.rmc_ids, "mg_rmc_gibraltar");
    assert_eq!(gib.rmc_defender_ids, "kingdom_defender");
    assert_eq!(gib.actor_spacing, ActorSpacing::Circle);
    assert_eq!(gib.actor_spacing_distance_bits, 1.5f32.to_bits());
    assert_eq!(gib.nodes, ["LandBorder".to_string(), "WaterBorder".to_string()]);
    assert_eq!(gib.location, [24.857548f32.to_bits(), 33.687256f32.to_bits(), 0]);
    assert_eq!(gib.links.len(), 7);
    assert_eq!(map.territories[gib.links[0].target].ids, "EasternCanada");
    assert_eq!(gib.links[0].rmc_attacker_ids, "kingdom_attacker_1");
    let kinds = |k| map.territories.iter().filter(|t| t.kind == k).count();
    assert_eq!(
        (
            kinds(TerritoryKind::Normal),
            kinds(TerritoryKind::Alias),
            kinds(TerritoryKind::Neutral),
            kinds(TerritoryKind::Background)
        ),
        (55, 10, 9, 42)
    );
    // alias territories are paired both ways and never link targets themselves
    for (i, t) in map.territories.iter().enumerate() {
        if t.kind == TerritoryKind::Alias {
            let p = t.alias_partner.unwrap();
            assert_eq!(map.territories[p].alias_partner, Some(i));
        }
        if !t.kind.has_links() {
            assert!(t.links.is_empty(), "{} links are not parsed natively", t.ids);
        }
        assert!(!t.actor_slots.is_empty());
    }
    let link_count: usize = map.territories.iter().map(|t| t.links.len()).sum();
    println!("links {link_count}");

    // MGDivision / armies / MGAI / commands
    assert_eq!(data.divisions.len(), 20);
    assert!(data.armies.len() >= 35, "{}", data.armies.len());
    let inf = data.armies.iter().find(|a| a.ids == "mg_jpn_army_infantry").unwrap();
    assert_eq!(inf.damage_bits, 0x3d4c_cccd);
    assert_eq!(inf.movement, 1);
    assert_eq!(data.divisions[inf.divisions[0]].ids, "jpn_division_infantry");
    let money = inf.stockpile_bonus.values.iter().find(|(n, _)| n == "money").unwrap().1;
    assert_eq!(money, 50f32.to_bits());
    let cap = data.armies.iter().find(|a| a.ids == "mg_jpn_capital").unwrap();
    assert!(cap.capturable && !cap.upgradable);
    assert_eq!(cap.movement, 0);
    assert_eq!(data.ai.len(), 5);
    let usa_ai = &data.ai[0];
    assert_eq!(usa_ai.ids, "usa_worldmap_mgai");
    assert_eq!(usa_ai.additional_attack_occupied_importance, 12);
    assert_eq!(usa_ai.purchase_army_probability_bits, 0.60f32.to_bits());
    assert_eq!(usa_ai.maps.len(), 1);
    assert_eq!(usa_ai.maps[0].capital.additional_importance, 40);
    assert_eq!(usa_ai.maps[0].default_territory.required_attack_strength_ratio_bits, 0.60f32.to_bits());
    let cmds: Vec<_> = data.commands.iter().map(|c| c.index).collect();
    assert_eq!(cmds, [0, 1, 2, 6, 10], "move attack startbattle endturn upgradeactorall");

    // DynamicCampaign
    let ids: Vec<_> = data.campaigns.iter().map(|c| c.ids.as_str()).collect();
    assert_eq!(
        ids,
        [
            "usa_dynamic_campaign",
            "great_britain_dynamic_campaign",
            "russia_dynamic_campaign",
            "germany_dynamic_campaign",
            "jpn_dynamic_campaign"
        ]
    );
    for c in &data.campaigns {
        assert_eq!(c.map, Some(0));
        assert_eq!(c.kingdoms.len(), 5, "{}", c.ids);
        assert_eq!(c.teams.len(), 2);
        assert_eq!(c.teams[0].ids, "axis_team");
        let names = |t: &aa_metagame::data::CampaignTeam| -> Vec<String> {
            t.kingdoms.iter().map(|&k| c.kingdoms[k].ids.clone()).collect()
        };
        assert_eq!(names(&c.teams[0]), ["kingdom_germany", "kingdom_jpn"]);
        assert_eq!(names(&c.teams[1]), ["kingdom_usa", "kingdom_great_britain", "kingdom_russia"]);
        assert!(c.events.is_empty(), "all shipped events are commented out");
        assert_eq!(c.initial_month_index, 11);
        assert_eq!(c.initial_year, 1941);
        assert_eq!(c.attacking_destroy_behavior, DestroyBehavior::Weakest);
        assert_eq!(c.defending_destroy_behavior, DestroyBehavior::Weakest);
        assert_eq!(c.default_kingdom, 0, "the player kingdom is listed first");
        for k in &c.kingdoms {
            assert_eq!(k.starting_resources, 50);
            assert_eq!(k.capital_captures, 2);
            assert!(k.territories.iter().any(|t| t.territory == k.capital), "{} owns its capital", k.ids);
            assert!(k.mgai.is_some());
        }
    }
    let usa = &data.campaigns[0];
    let japan = usa.kingdoms.iter().find(|k| k.ids == "kingdom_jpn").unwrap();
    assert_eq!(map.territories[japan.capital].ids, "Japan");
    assert_eq!(data.ai[japan.mgai.unwrap()].ids, "jpn_worldmap_mgai");
    assert_eq!(map.territories[japan.territories[0].territory].ids, "Japan");
    assert_eq!(japan.territories[0].liberates_to.map(|k| usa.kingdoms[k].ids.as_str()), Some("kingdom_jpn"));
    assert_eq!(
        japan.territories[0].actors,
        ["mg_jpn_capital", "mg_jpn_army_defensive", "mg_jpn_army_defensive"]
    );
    assert_eq!(japan.territories[1].liberates_to, None, "authored null");

    // Shop lists sorted by the original qsort: compare with the native fixture.
    let fx: Value = serde_json::from_slice(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../research/phase8-metagame/native-qsort.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let templates = [
        ("USADynamicCampaignShop", "kingdom_usa"),
        ("JPNDynamicCampaignShop", "kingdom_jpn"),
        ("GBRDynamicCampaignShop", "kingdom_great_britain"),
        ("GERDynamicCampaignShop", "kingdom_germany"),
        ("RUSDynamicCampaignShop", "kingdom_russia"),
    ];
    for shop in fx["shipped_shops"].as_array().unwrap() {
        let name = shop["template"].as_str().unwrap();
        let kingdom = templates.iter().find(|(t, _)| *t == name).unwrap().1;
        let k = usa.kingdoms.iter().find(|k| k.ids == kingdom).unwrap();
        for (kind, rows) in [
            ("Technology", &k.shop.technologies),
            ("Actor", &k.shop.actors),
            ("Division", &k.shop.divisions),
        ] {
            let list = &shop["lists"][kind];
            let authored = list["rows"].as_array().unwrap();
            let want: Vec<(String, i64)> = list["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| {
                    let r = &authored[i.as_u64().unwrap() as usize];
                    (r["ids"].as_str().unwrap().to_string(), r["cost"].as_i64().unwrap())
                })
                .collect();
            let got: Vec<(String, i64)> = rows
                .iter()
                .map(|r| (r.ids.clone().unwrap(), i64::from(r.cost)))
                .collect();
            assert_eq!(got, want, "{name} {kind}");
        }
        let all_costs: Vec<i64> = shop["lists"]["all"]["order"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| shop["lists"]["all"]["costs"][i.as_u64().unwrap() as usize].as_i64().unwrap())
            .collect();
        let got: Vec<i64> = k.shop.all.iter().map(|r| i64::from(r.cost)).collect();
        assert_eq!(got, all_costs, "{name} all");
    }
}

/// The oracle input research/phase8-metagame/metagame-data.json must equal
/// the definitions loaded now (every native-*.json fixture pins its sha256).
/// With `AA_METAGAME_EXPORT=1` the file is rewritten instead; after that every
/// oracle must be regenerated (README.txt RUN). Deterministic.
#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn export_definitions_for_oracles() {
    let data = load();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-metagame/metagame-data.json");
    let text = serde_json::to_string_pretty(&data).unwrap() + "\n";
    if std::env::var_os("AA_METAGAME_EXPORT").is_some_and(|v| v == "1") {
        std::fs::write(path, text).unwrap();
        return;
    }
    let on_disk = std::fs::read_to_string(path).unwrap();
    assert!(
        on_disk == text,
        "metagame-data.json is stale against the current loader/aa-rules: rerun with AA_METAGAME_EXPORT=1, then regenerate the oracles"
    );
}

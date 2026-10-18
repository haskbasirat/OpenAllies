//! Loader validations ported from the native metagame loaders, exercised by
//! patching single authored values in the shipped sources (requires the local
//! installation). These check the Rust port's behaviour against the native
//! messages and conditions read from the code; they are not Unicorn oracles.
use aa_metagame::Error;
use aa_metagame::battle::DestroyBehavior;
use aa_metagame::data::MetaGameData;
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use aa_rules::sources::{ResolvedFile, SourceLimits, V14Sources, sha256};
use aa_rules::tgi::Limits;
use std::path::PathBuf;
use std::sync::OnceLock;

struct Inputs {
    files: Vec<(String, aa_rules::sources::Origin, Vec<u8>)>,
    resources: Vec<String>,
    opaque: Vec<(String, Vec<u8>)>,
}

fn inputs() -> &'static Inputs {
    static I: OnceLock<Inputs> = OnceLock::new();
    I.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let mut s = V14Sources::open(
            root.join("../Data.rwd"),
            root.join("../reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits { max_member_bytes: 64 << 20 },
        )
        .unwrap();
        let paths: Vec<_> = s
            .catalog()
            .into_iter()
            .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
            .map(|r| r.logical_path)
            .collect();
        let mut files = Vec::new();
        for p in paths {
            let r = s.read(&p).unwrap();
            files.push((r.logical_path, r.origin, r.bytes));
        }
        let resources = aa_rules::resources::ResourceCatalog::load_v14(&mut s)
            .unwrap()
            .resources
            .iter()
            .map(|r| r.id.clone())
            .collect();
        let opaque = vec![(
            "Units/USA/usa_carrier.tgi".to_string(),
            s.read("Units/USA/usa_carrier.tgi").unwrap().bytes,
        )];
        Inputs { files, resources, opaque }
    })
}

/// Load with `from` replaced by `to` (exactly `count` occurrences) in `path`.
fn load_patched(path: &str, from: &str, to: &str, count: usize) -> Result<MetaGameData, Error> {
    let i = inputs();
    let files = i
        .files
        .iter()
        .map(|(p, origin, bytes)| {
            let bytes = if p.eq_ignore_ascii_case(path) {
                let text = String::from_utf8_lossy(bytes).into_owned();
                // shipped sources use CRLF line ends
                let (from, to) = (from.replace('\n', "\r\n"), to.replace('\n', "\r\n"));
                assert_eq!(text.matches(&from).count(), count, "patch anchor {from:?} in {path}");
                text.replacen(&from, &to, count).into_bytes()
            } else {
                bytes.clone()
            };
            ResolvedFile {
                logical_path: p.clone(),
                origin: origin.clone(),
                sha256: sha256(&bytes),
                bytes,
            }
        })
        .collect();
    let catalog = ActorCatalog::from_files(files, Limits::default());
    MetaGameData::from_catalog(&catalog, &i.resources, &i.opaque, Precision::Extended64)
}

fn native_error(r: Result<MetaGameData, Error>) -> String {
    match r {
        Err(Error::NativeData(m)) => m,
        Err(other) => panic!("expected native data error, got {other:?}"),
        Ok(_) => panic!("expected native data error, loaded successfully"),
    }
}

const CAMPAIGNS: &str = "MetaGame/mg_campaigns.tgi";
const MAP: &str = "MetaGame/mg_map.tgi";

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn initial_month_outside_1_to_12_is_fatal() {
    let m = native_error(load_patched(CAMPAIGNS, "initial_month = 12", "initial_month = 13", 1));
    assert_eq!(m, "Invalid initial month for dynamic campaign data");
    let m = native_error(load_patched(CAMPAIGNS, "initial_month = 12", "initial_month = 0", 1));
    assert_eq!(m, "Invalid initial month for dynamic campaign data");
    let ok = load_patched(CAMPAIGNS, "initial_month = 12", "initial_month = 1", 1).unwrap();
    assert_eq!(ok.campaigns[0].initial_month_index, 0);
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn destroy_behaviour_lookup_is_case_insensitive_and_fatal_when_unknown() {
    let ok = load_patched(
        CAMPAIGNS,
        "attacking_destroy_behavior = weakest",
        "attacking_destroy_behavior = MOST_Expensive",
        1,
    )
    .unwrap();
    assert_eq!(ok.campaigns[0].attacking_destroy_behavior, DestroyBehavior::MostExpensive);
    let m = native_error(load_patched(
        CAMPAIGNS,
        "defending_destroy_behavior = weakest",
        "defending_destroy_behavior = weakestx",
        1,
    ));
    assert_eq!(m, "Unknown IDS 'weakestx' specified in enum table");
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn team_member_must_be_a_campaign_kingdom() {
    let m = native_error(load_patched(
        CAMPAIGNS,
        "\t\tids = kingdom_germany\n\n\t\t[Kingdom]\n\t\tids = kingdom_jpn",
        "\t\tids = kingdom_germany\n\n\t\t[Kingdom]\n\t\tids = kingdom_italy",
        1,
    ));
    assert_eq!(m, "Kingdom with IDS 'kingdom_italy' not found for team 'axis_team'.");
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn capital_must_be_a_map_territory() {
    let m = native_error(load_patched(CAMPAIGNS, "capital_ids = Japan", "capital_ids = Atlantis", 1));
    assert_eq!(m, "Capital territory with IDS 'Atlantis' not found.");
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn two_kingdoms_cannot_own_one_territory() {
    // Japan's second record becomes Germany (owned by kingdom_germany).
    let m = native_error(load_patched(
        CAMPAIGNS,
        "\t\t[Territory]\n\t\t{\n\t\t\tids = Marshallislands\n\t\t}",
        "\t\t[Territory]\n\t\t{\n\t\t\tids = germany\n\t\t}",
        1,
    ));
    // Kingdoms are visited in campaign order (usa, jpn, ger, ...): the first
    // conflicting owner is kingdom_jpn, then kingdom_germany; the territory
    // name comes from the map (canonical spelling).
    assert_eq!(m, "2 kingdoms kingdom_jpn and kingdom_germany cannot own the same territory Germany");
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn minimap_blend_must_be_within_0_and_1() {
    let m = native_error(load_patched(MAP, "minimap_blend_amount = 0.4", "minimap_blend_amount = 1.5", 1));
    assert_eq!(m, "Minimap blend amount must be between 0 and 1");
    load_patched(MAP, "minimap_blend_amount = 0.4", "minimap_blend_amount = 1.0", 1).unwrap();
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn unknown_link_target_is_fatal() {
    let m = native_error(load_patched(
        MAP,
        "\t\t[Link]\n\t\tterritory_ids = EasternCanada\n\t\trmc_attacker_ids = kingdom_attacker_1\n\n\t\t[Link]\n\t\tterritory_ids = EasternUnitedStates1",
        "\t\t[Link]\n\t\tterritory_ids = Atlantis\n\t\trmc_attacker_ids = kingdom_attacker_1\n\n\t\t[Link]\n\t\tterritory_ids = EasternUnitedStates1",
        1,
    ));
    assert_eq!(m, "Territory link with IDS 'Atlantis' not found.");
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn rmc_attacker_must_be_an_rmc_kingdom_case_sensitive() {
    let m = native_error(load_patched(
        MAP,
        "\t\tterritory_ids = EasternCanada\n\t\trmc_attacker_ids = kingdom_attacker_1\n\n\t\t[Link]\n\t\tterritory_ids = EasternUnitedStates1",
        "\t\tterritory_ids = EasternCanada\n\t\trmc_attacker_ids = Kingdom_Attacker_1\n\n\t\t[Link]\n\t\tterritory_ids = EasternUnitedStates1",
        1,
    ));
    assert_eq!(m, "RMC attacker 'Kingdom_Attacker_1' is not a valid kingdom in the territory 'Gibraltar'");
}

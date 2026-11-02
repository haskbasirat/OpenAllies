//! The World War II campaign's data for the client: the metagame
//! definitions (`aa_metagame::data::MetaGameData`, the native loaders of
//! MetaGame/*.tgi) and what the strategic screens show of the original
//! data: the normal-aspect model of every metagame Thing (territory
//! doodads, map table props, armies), the kingdom colours
//! (Game/kingdom_colors.tgi), the map table props (mg_maptable.tgi), the
//! chooser art of every campaign (`choose_background_texture`,
//! `ChooseButtonVE`) and the factions (generals) of every nation.
//!
//! Read from the original rules catalog with the template resolution aa-play
//! uses (`aa_rules::actors::ActorCatalog::load_v14` over Data.rwd and the
//! v14 overlay). The catalog load takes seconds, so the result is cached in
//! `assets-cache/aa-bevy/campaign.json`, keyed by the source ledger.
use aa_metagame::data::MetaGameData;
use aa_numeric::Precision;
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::native::TypedValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

const PC: Precision = Precision::Double53;
/// Bump when the cached layout changes.
const CACHE_VERSION: &str = "aa-bevy-campaign-3";

/// One `ChooseButtonVE` of a campaign (the chooser's campaign button decals).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChooseVe {
    pub when: String,
    /// Data-relative, `%s` kept (the UI resolves the resolution folder).
    pub texture: String,
}

/// What the chooser shows of a campaign.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CampaignView {
    pub ids: String,
    pub choose_background: String,
    pub buttons: Vec<ChooseVe>,
    /// Campaign kingdom `icon` per kingdom (MetaGame-relative, `%s` kept).
    pub kingdom_icons: Vec<String>,
}

/// A prop of a map table (`MGMapTablePropTemplate`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PropView {
    pub object_ids: String,
    /// `location` after `prop_location_offset` / `prop_location_scale`.
    pub location: [f32; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TableView {
    pub ids: String,
    pub props: Vec<PropView>,
}

/// A faction (general) of a nation (Data/Generals, `Faction`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FactionView {
    pub ids: String,
    /// Localized keys.
    pub name: String,
    pub staging_name: String,
    pub description: String,
    /// `video_resource_ids`.
    pub video: String,
    /// `public` (default true): 473110 counts / 942F80 lists only these.
    pub public: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NationView {
    pub ids: String,
    pub name: String,
    /// The nation's `factions` list, authored order.
    pub factions: Vec<FactionView>,
}

/// The presentation side.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViewData {
    /// Thing IDS (lower case) -> Data-relative model of its normal aspect.
    pub models: BTreeMap<String, String>,
    /// Thing IDS (lower case) -> Data-relative model of `aspect_death`.
    pub death_models: BTreeMap<String, String>,
    /// Thing IDS (lower case) -> localized key of its `name`.
    pub names: BTreeMap<String, String>,
    /// KingdomColor IDS (lower case) -> RGB (0..1).
    pub colors: BTreeMap<String, [f32; 3]>,
    pub campaigns: Vec<CampaignView>,
    pub tables: Vec<TableView>,
    /// Nation IDS (lower case) -> nation.
    pub nations: BTreeMap<String, NationView>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignData {
    pub source_key: String,
    pub meta: MetaGameData,
    pub view: ViewData,
}

impl CampaignData {
    pub fn model(&self, ids: &str) -> Option<&str> {
        self.view.models.get(&ids.to_ascii_lowercase()).map(String::as_str)
    }
    pub fn thing_name(&self, ids: &str) -> Option<&str> {
        self.view.names.get(&ids.to_ascii_lowercase()).map(String::as_str)
    }
    pub fn color(&self, ids: &str) -> [f32; 3] {
        self.view.colors.get(&ids.to_ascii_lowercase()).copied().unwrap_or([1.0, 1.0, 1.0])
    }
    pub fn nation(&self, ids: &str) -> Option<&NationView> {
        self.view.nations.get(&ids.to_ascii_lowercase())
    }
}

fn value(node: &ResolvedNode<'_>, name: &str) -> Option<TypedValue> {
    match node.field(name, PC) {
        Ok(v) => v,
        Err(_) => node.plain_string_field(name).ok().flatten(),
    }
}
fn text(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    match value(node, name)? {
        TypedValue::String { value, .. } => {
            let v = value.trim().trim_matches('"').to_owned();
            (!v.is_empty() && !v.eq_ignore_ascii_case("null")).then_some(v)
        }
        _ => None,
    }
}
/// A localized field's authored key (5B5620 reads the quoted literal).
fn localized(node: &ResolvedNode<'_>, name: &str) -> String {
    node.localized(name).ok().flatten().map(|l| l.text).or_else(|| text(node, name)).unwrap_or_default()
}
fn text_with_folder(node: &ResolvedNode<'_>, name: &str) -> Option<(String, String)> {
    match value(node, name)? {
        TypedValue::String { value, source, .. } => {
            let v = value.trim().to_owned();
            if v.is_empty() || v.eq_ignore_ascii_case("null") {
                return None;
            }
            let path = source.path.replace('\\', "/");
            let folder = path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
            Some((v, folder))
        }
        _ => None,
    }
}
fn flag(node: &ResolvedNode<'_>, name: &str) -> Option<bool> {
    match value(node, name)? {
        TypedValue::Boolean { value, .. } => Some(value),
        TypedValue::Integer { value, .. } => Some(value != 0),
        TypedValue::String { value, .. } => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
fn point(node: &ResolvedNode<'_>, name: &str) -> Option<Vec<f32>> {
    match value(node, name)? {
        TypedValue::Point { bits, .. } => Some(bits.iter().map(|b| f32::from_bits(*b)).collect()),
        TypedValue::Scalar(s) => Some(vec![f32::from_bits(s.bits)]),
        _ => None,
    }
}
fn num(node: &ResolvedNode<'_>, name: &str) -> Option<f32> {
    match value(node, name)? {
        TypedValue::Scalar(s) => Some(f32::from_bits(s.bits)),
        TypedValue::Integer { value, .. } => Some(value as f32),
        TypedValue::String { value, .. } => value.trim().parse().ok(),
        _ => None,
    }
}

/// 4763C0: '/' is from the Data root, otherwise relative to the folder.
fn relative(folder: &str, value: &str) -> String {
    let value = value.replace('\\', "/");
    let folder = folder.trim_start_matches("Data/").trim_start_matches("data/");
    if let Some(v) = value.strip_prefix('/') {
        v.to_owned()
    } else if folder.is_empty() || folder.eq_ignore_ascii_case("data") {
        value
    } else {
        format!("{folder}/{value}")
    }
}

/// The model of an aspect (template_aspect.tgi: `model = %s.nif`, `%s` =
/// `resource_prefix` + `aspect_prefix`; '/' from the Data root, otherwise
/// relative to the assigning file).
fn aspect_model(aspect: &ResolvedNode<'_>, prefix: Option<&str>) -> Option<String> {
    let (model, folder) = text_with_folder(aspect, "model").unwrap_or_else(|| ("%s.nif".into(), String::new()));
    let model = model.replace('\\', "/");
    if model.contains("%s") {
        let mut p = prefix?.to_owned();
        if let Some(a) = text(aspect, "aspect_prefix") {
            p.push_str(&a);
        }
        return Some(model.replacen("%s", &p, 1).trim_start_matches('/').to_owned());
    }
    Some(relative(&folder, &model))
}

/// Normal and death models of a Thing's View: with `aspects_as_variations`
/// (49DC40, every metagame doodad and prop) the first `Aspect` is variant 0
/// (the `AspectNormal` the Object template gives them is `%s.nif`, not a
/// model); otherwise `AspectNormal`, else the `Aspect` whose IDS is
/// `aspect_normal`/`default`, else the first Aspect.
fn thing_models(catalog: &ActorCatalog, ids: &str) -> (Option<String>, Option<String>) {
    let Ok(actor) = catalog.resolved_actor(ids) else { return (None, None) };
    let Ok(Some(view)) = actor.child("View") else { return (None, None) };
    let prefix = text_with_folder(&view, "resource_prefix").map(|(v, folder)| relative(&folder, &v));
    let mut normal = None;
    if !flag(&view, "aspects_as_variations").unwrap_or(false)
        && let Ok(Some(a)) = view.child("AspectNormal")
    {
        normal = aspect_model(&a, prefix.as_deref());
    }
    let aspects = view.children("Aspect").unwrap_or_default();
    let mut death = None;
    let mut first = None;
    for a in &aspects {
        let ids = text(a, "IDS").unwrap_or_default().to_ascii_lowercase();
        let m = aspect_model(a, prefix.as_deref());
        if ids == "aspect_death" {
            death = m;
            continue;
        }
        if normal.is_none() && (ids == "aspect_normal" || ids == "default") {
            normal = m.clone();
        }
        if first.is_none() {
            first = m;
        }
    }
    (normal.or(first).or_else(|| prefix.map(|p| format!("{p}.nif"))), death)
}

/// `0x960950` over MetaGame/mg_maptable.tgi: each table's props with
/// `location` = (offset + location) * scale.
fn map_tables(bytes: Option<Vec<u8>>, notes: &mut Vec<String>) -> Vec<TableView> {
    use aa_rules::tgi::{Document, Item, Limits};
    let Some(bytes) = bytes else {
        notes.push("MetaGame/mg_maptable.tgi is missing".into());
        return Vec::new();
    };
    let Ok(doc) = Document::parse_native(bytes, Limits::default()) else {
        notes.push("MetaGame/mg_maptable.tgi does not parse".into());
        return Vec::new();
    };
    let scalars = |raw: &str| -> Vec<f32> { raw.split(',').filter_map(|p| aa_rules::native::scalar_literal(p.trim(), PC).ok()).map(f32::from_bits).collect() };
    let field = |node: &aa_rules::tgi::Node, name: &str| -> Option<String> {
        node.items.iter().find_map(|item| match item {
            Item::Property(p) if doc.property_name(p).is_some_and(|n| n.eq_ignore_ascii_case(name)) => doc.property_value(p).map(|v| v.trim().trim_matches('"').to_string()),
            _ => None,
        })
    };
    let mut tables = Vec::new();
    for item in &doc.items {
        let Item::Node(node) = item else { continue };
        if !doc.header_name(&node.header).is_some_and(|n| n.eq_ignore_ascii_case("MGMapTable")) {
            continue;
        }
        let offset = field(node, "prop_location_offset").map(|v| scalars(&v)).unwrap_or_default();
        let scale = field(node, "prop_location_scale").and_then(|v| scalars(&v).first().copied()).unwrap_or(1.0);
        let mut table = TableView { ids: field(node, "IDS").unwrap_or_default(), props: Vec::new() };
        for child in &node.items {
            let Item::Node(prop) = child else { continue };
            if !doc.header_name(&prop.header).is_some_and(|n| n.eq_ignore_ascii_case("Prop")) {
                continue;
            }
            let l = field(prop, "location").map(|v| scalars(&v)).unwrap_or_default();
            let at = |i: usize| (offset.get(i).copied().unwrap_or(0.0) + l.get(i).copied().unwrap_or(0.0)) * scale;
            table.props.push(PropView { object_ids: field(prop, "object_ids").unwrap_or_default(), location: [at(0), at(1)] });
        }
        tables.push(table);
    }
    tables
}

fn extract(catalog: &ActorCatalog, meta: &MetaGameData, map_table_file: Option<Vec<u8>>) -> ViewData {
    let mut v = ViewData::default();
    // Things of the strategic map: territory doodads, armies, props
    let mut things: Vec<String> = Vec::new();
    for m in &meta.maps {
        things.extend(m.territories.iter().map(|t| t.object_ids.clone()));
    }
    things.extend(meta.armies.iter().map(|a| a.ids.clone()));
    // map tables (mg_maptable.tgi, MGMapTable): 960950 reads the [Prop] list,
    // which the rules catalog does not declare as a list
    v.tables = map_tables(map_table_file, &mut v.notes);
    things.extend(v.tables.iter().flat_map(|t| t.props.iter().map(|p| p.object_ids.clone())));
    things.sort_by_key(|s| s.to_ascii_lowercase());
    things.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    for ids in &things {
        let (normal, death) = thing_models(catalog, ids);
        match normal {
            Some(m) => {
                v.models.insert(ids.to_ascii_lowercase(), m);
            }
            None => v.notes.push(format!("no model for {ids}")),
        }
        if let Some(d) = death {
            v.death_models.insert(ids.to_ascii_lowercase(), d);
        }
        if let Ok(actor) = catalog.resolved_actor(ids) {
            let name = localized(&actor, "name");
            if !name.is_empty() {
                v.names.insert(ids.to_ascii_lowercase(), name);
            }
        }
    }
    // kingdom colours
    for c in catalog.resolved_definitions("KingdomColor").unwrap_or_default() {
        if let (Some(ids), Some(rgb)) = (text(&c, "IDS"), point(&c, "color")) {
            let g = |i: usize| rgb.get(i).copied().unwrap_or(1.0);
            v.colors.insert(ids.to_ascii_lowercase(), [g(0), g(1), g(2)]);
        }
    }
    // campaigns (DynamicCampaign): chooser art
    for c in catalog.resolved_definitions("DynamicCampaign").unwrap_or_default() {
        let mut view = CampaignView {
            ids: text(&c, "IDS").unwrap_or_default(),
            choose_background: text(&c, "choose_background_texture").unwrap_or_default(),
            ..Default::default()
        };
        for b in c.children("ChooseButtonVE").unwrap_or_default() {
            view.buttons.push(ChooseVe { when: text(&b, "when").unwrap_or_default(), texture: text(&b, "texture").unwrap_or_default() });
        }
        for k in c.children("Kingdom").unwrap_or_default() {
            view.kingdom_icons.push(text(&k, "icon").unwrap_or_default());
        }
        v.campaigns.push(view);
    }
    // factions by IDS, then the nations' lists
    let mut factions: BTreeMap<String, FactionView> = BTreeMap::new();
    for f in catalog.resolved_definitions("Faction").unwrap_or_default() {
        let Some(ids) = text(&f, "IDS") else { continue };
        factions.insert(
            ids.to_ascii_lowercase(),
            FactionView {
                ids: ids.clone(),
                name: localized(&f, "name"),
                staging_name: localized(&f, "staging_name"),
                description: localized(&f, "description"),
                video: text(&f, "video_resource_ids").unwrap_or_default(),
                public: flag(&f, "public").unwrap_or(true),
            },
        );
    }
    for n in catalog.resolved_definitions("Nation").unwrap_or_default() {
        let Some(ids) = text(&n, "IDS") else { continue };
        let list = text(&n, "factions").unwrap_or_default();
        let mut nation = NationView { ids: ids.clone(), name: localized(&n, "name"), factions: Vec::new() };
        for f in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match factions.get(&f.to_ascii_lowercase()) {
                Some(fv) => nation.factions.push(fv.clone()),
                None => v.notes.push(format!("nation {ids}: unknown faction {f}")),
            }
        }
        v.nations.insert(ids.to_ascii_lowercase(), nation);
    }
    v
}

fn cache_path(game_root: &Path) -> std::path::PathBuf {
    crate::effects::data::cache_dir(game_root).join("campaign.json")
}

/// The data from the cache when it matches the sources, else from the
/// original catalog (seconds), then cached.
pub fn load(game_root: &Path) -> anyhow::Result<CampaignData> {
    let key = format!("{CACHE_VERSION}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256);
    let path = cache_path(game_root);
    if std::env::var_os("AA_BEVY_CAMPAIGN_NO_CACHE").is_none()
        && let Ok(bytes) = std::fs::read(&path)
        && let Ok(d) = serde_json::from_slice::<CampaignData>(&bytes)
        && d.source_key == key
    {
        return Ok(d);
    }
    let mut sources = aa_play::data::Sources::open(game_root)?;
    let files = &mut sources.files;
    let catalog = ActorCatalog::load_v14(files).map_err(|e| anyhow::anyhow!("loading the original rules catalog: {e}"))?;
    let resources = aa_rules::resources::ResourceCatalog::load_v14(files).map_err(|e| anyhow::anyhow!("resource catalog: {e}"))?;
    let names: Vec<String> = resources.resources.iter().map(|r| r.id.clone()).collect();
    let mut opaque = Vec::new();
    for d in &catalog.diagnostics {
        if d.message.starts_with("TGI syntax") {
            if let Ok(f) = files.read(&d.source.path) {
                opaque.push((d.source.path.clone(), f.bytes));
            }
        }
    }
    // the oracle input's precision (research/phase8-metagame/metagame-data.json)
    let meta = MetaGameData::from_catalog(&catalog, &names, &opaque, Precision::Extended64)
        .map_err(|e| anyhow::anyhow!("metagame definitions: {e}"))?;
    let map_table_file = files.read("MetaGame/mg_maptable.tgi").ok().map(|f| f.bytes);
    let view = extract(&catalog, &meta, map_table_file);
    let data = CampaignData { source_key: key, meta, view };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(bytes) = serde_json::to_vec(&data) {
        let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(data)
}

type Shared = Mutex<Option<Result<Arc<CampaignData>, String>>>;
static DATA: OnceLock<Shared> = OnceLock::new();

/// Start loading on a worker (once).
pub fn warm(game_root: std::path::PathBuf) {
    let cell = DATA.get_or_init(|| Mutex::new(None));
    if cell.lock().map(|g| g.is_some()).unwrap_or(true) {
        return;
    }
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    rayon::spawn(move || {
        let t = std::time::Instant::now();
        let r = load(&game_root).map(Arc::new).map_err(|e| format!("{e:#}"));
        match &r {
            Ok(d) => bevy::log::info!("campaign data: {} campaigns in {:.1} s ({} notes)", d.meta.campaigns.len(), t.elapsed().as_secs_f32(), d.view.notes.len()),
            Err(e) => bevy::log::error!("campaign data: {e}"),
        }
        if let Some(cell) = DATA.get()
            && let Ok(mut g) = cell.lock()
        {
            *g = Some(r);
        }
    });
}

/// The data once loaded (`None` while loading).
pub fn ready() -> Option<Result<Arc<CampaignData>, String>> {
    DATA.get().and_then(|c| c.lock().ok().and_then(|g| g.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_table_props_are_960950() {
        let text = "[MGMapTable]\n{\n\tIDS = worldmaptable\n\tpoint3 prop_location_offset = 1, 2, 0\n\tfixed prop_location_scale = 2.0\n\t[Prop]\n\t{\n\t\tobject_ids = mgprop_Globe\n\t\tlocation = 10.5, 20\n\t}\n\t[Prop]\n\t{\n\t\tobject_ids = mgprop_Chair05\n\t\tlocation = 0, 0\n\t}\n}\n";
        let mut notes = Vec::new();
        let tables = map_tables(Some(text.as_bytes().to_vec()), &mut notes);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].ids, "worldmaptable");
        assert_eq!(tables[0].props.len(), 2);
        assert_eq!(tables[0].props[0].object_ids, "mgprop_Globe");
        assert_eq!(tables[0].props[0].location, [23.0, 44.0]);
        assert_eq!(tables[0].props[1].location, [2.0, 4.0]);
    }
}

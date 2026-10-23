//! The original game data the staging screen and its dialogs read, from
//! the user's Data folder (and the v14 overlay): nations and their factions
//! (Data/Nations, Data/Generals), computer opponents (Data/SAI), handicaps,
//! kingdom colours, world rules (Data/Game), random map types
//! (Data/RandomMap on Data/Templates/template_rmc*.tgi) and the scenario
//! maps (Data/Maps/**/*.RMP headers and their .TGI text tables).
//!
//! Template inheritance (`template=` / `Inherit=`) is resolved structurally:
//! the template's fields and children first, the node's own on top.
use super::datafile::{self, Node};
use aa_ui::fs::DataRoot;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Faction {
    pub ids: String,
    /// `staging_name`, else `name` (localization key).
    pub name: String,
    pub selectable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Nation {
    /// "nation_usa".
    pub ids: String,
    /// "usa" (aa-play's nation id).
    pub short: String,
    pub name: String,
    pub flag_icon: Option<String>,
    pub playable: bool,
    pub factions: Vec<Faction>,
}

#[derive(Clone, Debug, Default)]
pub struct Sai {
    pub ids: String,
    pub name: String,
    pub nation: Option<String>,
    pub faction: Option<String>,
    pub selectable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct WorldRule {
    pub ids: String,
    pub name: String,
    /// bool / int / fixed / enum / enum_specified.
    pub kind: String,
    pub default: String,
    pub use_slider: bool,
    pub slider: (f32, f32, f32),
    pub editbox_max: Option<f32>,
    pub never_editable: bool,
    pub always_editable: bool,
    pub enum_values: Vec<(String, String)>,
    pub true_name: String,
    pub false_name: String,
}

#[derive(Clone, Debug, Default)]
pub struct RmcKingdom {
    pub ids: String,
    pub name: String,
    pub color: String,
    pub independent: bool,
    pub playable: bool,
    pub required: bool,
    pub team: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RmcTerrain {
    pub terrain_ids: String,
    pub name: String,
    pub range: (f32, f32),
    pub show_ui: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Rmc {
    pub ids: String,
    pub name: String,
    pub description: String,
    pub list_in_staging: bool,
    pub reference_size: f32,
    /// (width, height, recommended min, max).
    pub sizes: Vec<(f32, f32, i32, i32)>,
    pub kingdoms: Vec<RmcKingdom>,
    pub terrains: Vec<RmcTerrain>,
    /// `[Rules]/[Rule] ids default show_ui`.
    pub rules: Vec<(String, String, bool)>,
    /// `[Team]` (ids, name key).
    pub teams: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct MapKingdom {
    pub ids: String,
    /// The map's localized kingdom name ("Side 1").
    pub name: String,
    pub nation: Option<String>,
    pub faction: Option<String>,
    pub color: String,
    pub team: String,
    pub independent: bool,
    pub playable: bool,
    pub required: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MapInfo {
    /// Data-relative path ("Maps/(2v2)Berlin.RMP").
    pub path: String,
    /// File title without extension ("(2v2)Berlin"), aa-play's map id.
    pub title: String,
    pub name: String,
    pub description: String,
    pub size: (i32, i32),
    pub kingdoms: Vec<MapKingdom>,
    /// (ids, name).
    pub teams: Vec<(String, String)>,
    /// The map's own world rules (MAP_HEADER WorldRules v4): IDS, native
    /// kind (0 bool byte, 1 int, 2 fixed as f32 bits, 3 enum value index)
    /// and raw value.
    pub rules: Vec<(String, u32, u32)>,
}

pub struct GameData {
    pub nations: Vec<Nation>,
    pub sais: Vec<Sai>,
    /// (ids, name key).
    pub handicaps: Vec<(String, String)>,
    /// ids -> rgb.
    pub colors: Vec<(String, [f32; 3])>,
    pub rules: Vec<WorldRule>,
    pub rmcs: Vec<Rmc>,
    templates: HashMap<String, Node>,
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn point3(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| [p[0], p[1], p[2]])
}

fn bool_of(v: Option<&str>, default: bool) -> bool {
    v.map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")).unwrap_or(default)
}

/// Data-relative files with extension `ext` in `dir` (every layer), and in
/// its subfolders when `recursive`.
pub fn files_with(data: &DataRoot, dir: &str, ext: &str, recursive: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![dir.trim_end_matches('/').to_string()];
    while let Some(d) = dirs.pop() {
        for name in data.list_dir(&d) {
            if name.to_ascii_lowercase().ends_with(ext) {
                out.push(format!("{d}/{name}"));
            }
        }
        if recursive {
            if let Some(Ok(rd)) = data.find(&d).map(std::fs::read_dir) {
                for e in rd.flatten().filter(|e| e.path().is_dir()) {
                    if let Ok(n) = e.file_name().into_string() {
                        dirs.push(format!("{d}/{n}"));
                    }
                }
            }
        }
    }
    out.sort_by_key(|s| s.to_ascii_lowercase());
    out
}

fn tgi_files(data: &DataRoot, dir: &str, recursive: bool) -> Vec<String> {
    files_with(data, dir, ".tgi", recursive)
}

impl GameData {
    pub fn load(data: &DataRoot) -> Self {
        let mut templates = HashMap::new();
        for f in tgi_files(data, "Templates", false) {
            for n in datafile::load(data, &f) {
                if n.name.eq_ignore_ascii_case("Template") {
                    if let Some(name) = n.args.first() {
                        templates.insert(lower(name), n.clone());
                    }
                }
            }
        }
        let mut g = GameData { nations: Vec::new(), sais: Vec::new(), handicaps: Vec::new(), colors: Vec::new(), rules: Vec::new(), rmcs: Vec::new(), templates };
        g.load_factions_and_nations(data);
        g.load_sais(data);
        for n in datafile::load(data, "Game/handicaps.tgi").iter().filter(|n| n.name.eq_ignore_ascii_case("Handicap")) {
            g.handicaps.push((n.get("IDS").unwrap_or_default().to_string(), n.get("name").unwrap_or_default().to_string()));
        }
        for n in datafile::load(data, "Game/kingdom_colors.tgi").iter().filter(|n| n.name.eq_ignore_ascii_case("KingdomColor")) {
            if let Some(c) = n.get("color").and_then(point3) {
                g.colors.push((lower(n.get("IDS").unwrap_or_default()), c));
            }
        }
        for f in ["Game/world_rules.tgi", "Game/world_rules_aa.tgi"] {
            for n in datafile::load(data, f).iter().filter(|n| n.name.eq_ignore_ascii_case("WorldRule")) {
                let n = g.resolve(n);
                let ids = n.get("IDS").unwrap_or_default().to_string();
                let rule = WorldRule {
                    ids: ids.clone(),
                    name: n.get("name").unwrap_or_default().to_string(),
                    kind: lower(n.get("type").unwrap_or("bool")),
                    default: n.get("default").unwrap_or_default().to_string(),
                    use_slider: bool_of(n.get("use_slider"), false),
                    slider: (n.float("slider_min").unwrap_or(0.0), n.float("slider_max").unwrap_or(100.0), n.float("step_size").unwrap_or(1.0)),
                    editbox_max: n.float("editbox_max_value"),
                    never_editable: bool_of(n.get("never_editable"), false),
                    always_editable: bool_of(n.get("always_editable"), false),
                    enum_values: n.children_named("EnumValue").map(|e| (e.get("IDS").unwrap_or_default().to_string(), e.get("name").unwrap_or_default().to_string())).collect(),
                    true_name: n.get("true_name").unwrap_or_default().to_string(),
                    false_name: n.get("false_name").unwrap_or_default().to_string(),
                };
                let mut rule = rule;
                // "type = enum_specified": only DifficultyLevel is known
                // (0x4909DE): its values are the [DifficultyLevel] nodes of
                // Game/difficulty_level.tgi (easy / medium / hard)
                if rule.kind == "enum_specified" && rule.enum_values.is_empty() && ids.eq_ignore_ascii_case("DifficultyLevel") {
                    rule.enum_values = datafile::load(data, "Game/difficulty_level.tgi")
                        .iter()
                        .filter(|e| e.name.eq_ignore_ascii_case("DifficultyLevel"))
                        .map(|e| (e.get("IDS").unwrap_or_default().to_string(), e.get("name").unwrap_or_default().to_string()))
                        .collect();
                }
                match g.rules.iter_mut().find(|r| r.ids.eq_ignore_ascii_case(&ids)) {
                    Some(r) => *r = rule,
                    None => g.rules.push(rule),
                }
            }
        }
        g.load_rmcs(data);
        g
    }

    /// Fields and children of the template chain, then the node's own.
    pub fn resolve(&self, n: &Node) -> Node {
        self.resolve_depth(n, 0)
    }

    fn resolve_depth(&self, n: &Node, depth: u32) -> Node {
        let parent = n.attrs.get("template").or(n.attrs.get("inherit")).map(|s| lower(s));
        let mut out = match parent.and_then(|p| self.templates.get(&p)).filter(|_| depth < 16) {
            Some(t) => self.resolve_depth(t, depth + 1),
            None => Node::default(),
        };
        out.name = n.name.clone();
        out.args = n.args.clone();
        out.attrs = n.attrs.clone();
        for (k, v) in &n.fields {
            match out.fields.iter_mut().find(|(x, _)| x.eq_ignore_ascii_case(k)) {
                Some(f) => f.1 = v.clone(),
                None => out.fields.push((k.clone(), v.clone())),
            }
        }
        for c in &n.children {
            out.children.push(self.resolve_depth(c, depth + 1));
        }
        out
    }

    fn load_factions_and_nations(&mut self, data: &DataRoot) {
        let mut factions: HashMap<String, Faction> = HashMap::new();
        for f in tgi_files(data, "Generals", true) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Faction")) {
                let n = self.resolve(n);
                let ids = lower(n.get("IDS").unwrap_or_default());
                let name = n.get("staging_name").filter(|s| !s.is_empty()).or(n.get("name")).unwrap_or_default().to_string();
                // template_sai / template_faction `public`: false hides it in staging
                factions.insert(ids.clone(), Faction { ids, name, selectable: bool_of(n.get("public"), true) });
            }
        }
        for f in tgi_files(data, "Nations", false) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Nation")) {
                let n = self.resolve(n);
                let ids = lower(n.get("IDS").unwrap_or_default());
                let short = ids.strip_prefix("nation_").unwrap_or(&ids).to_string();
                let list: Vec<Faction> = n
                    .get("factions")
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| lower(s.trim()))
                    .filter(|s| !s.is_empty())
                    .map(|id| factions.get(&id).cloned().unwrap_or(Faction { ids: id.clone(), name: id, selectable: true }))
                    .collect();
                self.nations.push(Nation {
                    ids,
                    short,
                    name: n.get("name").unwrap_or_default().to_string(),
                    flag_icon: n.get("flag_icon").map(str::to_string),
                    // template_nation `flag selectable = true`
                    playable: bool_of(n.get("selectable"), true),
                    factions: list,
                });
            }
        }
    }

    fn load_sais(&mut self, data: &DataRoot) {
        for f in tgi_files(data, "SAI", false) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("SAI")) {
                let n = self.resolve(n);
                let ids = n.get("IDS").unwrap_or_default().to_string();
                if ids.is_empty() || self.sais.iter().any(|s| s.ids.eq_ignore_ascii_case(&ids)) {
                    continue;
                }
                self.sais.push(Sai {
                    ids,
                    name: n.get("name").unwrap_or_default().to_string(),
                    // [Nation] IDS / [Faction] ids children; `public`
                    nation: n.child("Nation").and_then(|c| c.get("IDS")).map(|s| lower(s)),
                    faction: n.child("Faction").and_then(|c| c.get("IDS")).map(|s| lower(s)),
                    selectable: bool_of(n.get("public"), true),
                });
            }
        }
    }

    fn load_rmcs(&mut self, data: &DataRoot) {
        // terrain type names (Data/Terrain/**/<ids>.tgi)
        let mut terrain_names: HashMap<String, String> = HashMap::new();
        for f in tgi_files(data, "Terrain", true) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Terrain")) {
                if let (Some(ids), Some(name)) = (n.get("IDS"), n.get("name")) {
                    terrain_names.insert(lower(ids), name.to_string());
                }
            }
        }
        for f in tgi_files(data, "RandomMap", false) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("RMC")) {
                let n = self.resolve(n);
                let mut rmc = Rmc {
                    ids: n.get("IDS").unwrap_or_default().to_string(),
                    name: n.get("name").unwrap_or_default().to_string(),
                    description: n.get("description").unwrap_or_default().to_string(),
                    list_in_staging: bool_of(n.get("list_in_staging"), true),
                    reference_size: n.float("reference_size").unwrap_or(384.0),
                    ..Default::default()
                };
                for c in &n.children {
                    let name = c.name.to_ascii_lowercase();
                    match name.as_str() {
                        "mapsize" => rmc.sizes.push((
                            c.float("width").unwrap_or(0.0),
                            c.float("height").unwrap_or(0.0),
                            c.int("recommended_kingdoms_min").unwrap_or(-1),
                            c.int("recommended_kingdoms_max").unwrap_or(-1),
                        )),
                        "kingdom" => rmc.kingdoms.push(RmcKingdom {
                            ids: c.get("IDS").unwrap_or_default().to_string(),
                            name: c.get("name").unwrap_or_default().to_string(),
                            color: lower(c.get("color").unwrap_or_default()),
                            independent: bool_of(c.get("independent"), false),
                            playable: bool_of(c.get("playable"), false),
                            required: bool_of(c.get("required"), false),
                            team: c.get("team").filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("null")).map(str::to_string),
                        }),
                        "terrainblob" | "terrainstreak" => {
                            let range: Vec<f32> = c.get("range").unwrap_or("0,0").split(',').filter_map(|x| x.trim().parse().ok()).collect();
                            let ids = lower(c.get("terrain_ids").unwrap_or_default());
                            rmc.terrains.push(RmcTerrain {
                                name: terrain_names.get(&ids).cloned().unwrap_or_else(|| ids.clone()),
                                terrain_ids: ids,
                                range: (range.first().copied().unwrap_or(0.0), range.get(1).copied().unwrap_or(0.0)),
                                show_ui: bool_of(c.get("show_ui"), false),
                                locked: bool_of(c.get("locked"), false),
                            });
                        }
                        "team" => rmc.teams.push((c.get("IDS").unwrap_or_default().to_string(), c.get("name").unwrap_or_default().to_string())),
                        "rules" => {
                            for r in c.children_named("Rule") {
                                rmc.rules.push((
                                    r.get("IDS").unwrap_or_default().to_string(),
                                    r.get("default").unwrap_or_default().to_string(),
                                    bool_of(r.get("show_ui"), true),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                self.rmcs.push(rmc);
            }
        }
    }

    pub fn color(&self, ids: &str) -> [f32; 3] {
        self.colors.iter().find(|(i, _)| i.eq_ignore_ascii_case(ids)).map(|(_, c)| *c).unwrap_or([1.0, 1.0, 1.0])
    }

    pub fn nation(&self, short_or_ids: &str) -> Option<&Nation> {
        let k = lower(short_or_ids);
        self.nations.iter().find(|n| n.ids == k || n.short == k)
    }
}

/// The map's own text table (Data/Maps/<title>.TGI `[Text]`).
fn map_strings(data: &DataRoot, path: &str) -> HashMap<String, String> {
    let tgi = match path.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.TGI"),
        None => format!("{path}.TGI"),
    };
    let mut out = HashMap::new();
    for n in datafile::load(data, &tgi).iter().filter(|n| n.name.eq_ignore_ascii_case("Text")) {
        for (k, v) in &n.fields {
            out.insert(lower(k), v.clone());
        }
    }
    out
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v).trim_end_matches('\0').to_string()
}

/// Read one scenario map's header (MAP_HEADER chunk) and text table.
pub fn read_map(data: &DataRoot, path: &str) -> Option<MapInfo> {
    let bytes = data.read(path).ok()?;
    let archive = aa_mapworld::archive::MapArchive::from_bytes(bytes).ok()?;
    let header = aa_mapworld::header::MapHeader::decode(&archive.decode_chunk("MAP_HEADER", 64 << 20).ok()?).ok()?;
    let strings = map_strings(data, path);
    let text = |key: &str| -> String {
        let k = lower(key.trim_start_matches('#'));
        strings.get(&k).cloned().unwrap_or_else(|| key.to_string())
    };
    let title = path.rsplit('/').next().unwrap_or(path);
    let title = title.rsplit_once('.').map(|(s, _)| s).unwrap_or(title).to_string();
    let name_key = utf16(&header.name_key);
    let name = strings.get("map_name").cloned().or_else(|| strings.get(&lower(&name_key)).cloned()).unwrap_or_else(|| title.clone());
    let description = strings.get("map_description").cloned().unwrap_or_default();
    let extents = header.display_extents_bits.or(Some(header.creator.world_extents_bits)).map(|b| (f32::from_bits(b[0]) as i32, f32::from_bits(b[1]) as i32)).unwrap_or((0, 0));
    let p = &header.parameters;
    if std::env::var_os("AA_UI_DUMP_MAPS").is_some() {
        for (label, list) in [("header", &header.parameters), ("creator", &header.creator.parameters)] {
            for k in &list.kingdoms {
                eprintln!(
                    "{path} {label} {} b10={} b28={} b29={} b31={} b32={} team={} nation={} color={} display={}",
                    utf16(&k.id), k.byte_10, k.byte_28, k.byte_29, k.byte_31, k.byte_32, utf16(&k.string_14), utf16(&k.nation), utf16(&k.color), utf16(&k.display_key)
                );
            }
        }
    }
    let kingdoms = p
        .kingdoms
        .iter()
        .map(|k| {
            let opt = |v: &[u16]| Some(lower(&utf16(v))).filter(|s| !s.is_empty() && s != "null");
            MapKingdom {
                ids: utf16(&k.id),
                name: text(&utf16(&k.display_key)),
                nation: opt(&k.nation),
                faction: opt(&k.faction),
                color: lower(&utf16(&k.color)),
                team: utf16(&k.string_14),
                independent: k.byte_10 != 0,
                playable: k.byte_28 != 0,
                required: k.byte_29 != 0,
            }
        })
        .collect();
    let teams = p.teams.iter().map(|t| (utf16(&t.id), text(&utf16(&t.display_key)))).collect();
    let rules = header.rules.rules.iter().map(|r| (utf16(&r.key), r.kind, r.value)).collect();
    Some(MapInfo { path: path.to_string(), title, name, description, size: extents, kingdoms, teams, rules })
}

/// The Scenario Map scan (`0x68E340`): the `Maps/*.RMP` files; the find
/// loop skips directories (Campaign, DeveloperMaps, TrailerMaps).
pub fn list_maps(data: &DataRoot) -> Vec<String> {
    files_with(data, "Maps", ".rmp", false)
}

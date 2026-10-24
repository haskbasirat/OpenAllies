//! Actor and kingdom names the HUD shows, from the original data:
//! * an actor's name (`0x4D7F40` SetName keeps it as the string `name_<GTID>`
//!   of the world's text table; GetName, vtable +0x24, falls back to the
//!   definition's display name): a scenario map's actors take
//!   `name_<serialized GTID>` from the map's text table (Maps/<map>.TGI,
//!   e.g. `name_11613 = "Kursk"`); on a random map every city center (a
//!   Thing with a CityComponent and `name_list_ids`) gets a name from its
//!   NameList (Data/Namelists) at creation (`0x4F4A20` AssignName: a
//!   random index, then the first one of that list not given yet, so the
//!   names of one list do not repeat);
//! * a kingdom's display name: the players' names, and for the independent
//!   kingdoms the session record's name (RMC `[Kingdom] name`, e.g.
//!   side_enemy = "Enemies"; a scenario map's header `independent_N` text,
//!   e.g. "Civilian (Aggressive)").
//!
//! * a city center's name floats above it: its WhenCreated `city_name_vfx`
//!   (Effects/text_effects.tgi, VFXObjectName: the actor's name in
//!   `template_vfx_VFXObjectName_format|s`, colour 1,1,1, font
//!   large_outline, permanent, risen 10 m over its base), shown while the
//!   UVar ViewShowVFXObjectName (the options' "Names" toggle, default on)
//!   is set and the city is drawn (fog), under the HUD.
//!
//! Compare against the original: the random-map draw uses a seed of this
//! client (the original draws from the world RNG at creation), so the names
//! are as random but not the same ones the original would pick.
use super::datafile;
use crate::session::Battle;
use aa_play::ActorId;
use aa_ui::fs::DataRoot;
use std::collections::HashMap;

#[derive(Default)]
pub struct Names {
    /// The battle the names were built for (map id, actor count).
    built: Option<(String, usize)>,
    actors: HashMap<ActorId, String>,
    /// Kingdom name (aa-play id) -> display name.
    pub kingdoms: HashMap<String, String>,
}

impl Names {
    pub fn actor(&self, id: ActorId) -> Option<&str> {
        self.actors.get(&id).map(String::as_str)
    }

    /// The name a building shows: its own, else that of the city whose
    /// zone holds it (the SettlementDisplay names the settlement, not the
    /// block).
    pub fn building(&self, w: &aa_play::World, id: ActorId) -> Option<&str> {
        if let Some(n) = self.actor(id) {
            return Some(n);
        }
        let a = w.actor(id)?;
        self.actors
            .iter()
            .filter_map(|(c, n)| w.actor(*c).filter(|x| x.alive && w.data.def(x.def).city).map(|x| (x, n)))
            .filter(|(x, _)| x.pos.truncate().distance(a.pos.truncate()) <= w.data.def(x.def).city_zone_radius)
            .min_by(|p, q| p.0.pos.truncate().distance(a.pos.truncate()).total_cmp(&q.0.pos.truncate().distance(a.pos.truncate())))
            .map(|(_, n)| n.as_str())
    }

    /// A kingdom's display name.
    pub fn kingdom<'a>(&'a self, name: &'a str) -> &'a str {
        self.kingdoms.get(&name.to_ascii_lowercase()).map(String::as_str).unwrap_or(name)
    }

    /// Build the actor names once per battle (and again when actors were
    /// created since, e.g. a city rebuilt).
    pub fn update(&mut self, data: &DataRoot, strings: &aa_ui::resources::StringTable, b: &Battle) {
        let w = &b.world;
        let key = (w.map.id.clone(), w.actors.len());
        if self.built.as_ref() == Some(&key) {
            return;
        }
        let first = self.built.as_ref().is_none_or(|(m, _)| *m != w.map.id);
        self.built = Some(key);
        if first {
            self.actors.clear();
        }
        let resolve = |k: &str| if k.starts_with('#') { strings.resolve(k) } else { k.to_string() };
        // a scenario map's text table
        let path = format!("Maps/{}.RMP", w.map.id);
        if data.exists(&path) {
            let table = map_table(data, &path);
            for a in w.actors.iter().filter(|a| a.alive) {
                if self.actors.contains_key(&a.id) {
                    continue;
                }
                let Some(p) = w.books.actor(a.id).and_then(|s| s.placed).and_then(|i| w.map.placed.get(i)) else { continue };
                if let Some(n) = table.get(&format!("name_{}", p.serialized_id)) {
                    self.actors.insert(a.id, resolve(n));
                }
            }
            return;
        }
        // a random map: the city centers' name lists
        let (lists, centers) = name_lists(data);
        // names already given (also to cities named before)
        let mut used: std::collections::HashSet<String> = self.actors.values().cloned().collect();
        let seed = w.map.id.bytes().fold(0x9E37_79B9u64, |h, c| h.rotate_left(5) ^ u64::from(c));
        for a in w.actors.iter().filter(|a| a.alive) {
            let def = w.data.def(a.def);
            if !def.city {
                continue;
            }
            let Some(list_ids) = centers.get(&def.id.to_ascii_lowercase()) else { continue };
            let Some(names) = lists.get(list_ids).filter(|l| !l.is_empty()) else { continue };
            if self.actors.contains_key(&a.id) {
                continue;
            }
            // 0x4F4A20: the drawn index, else the first one not given yet
            let mut x = seed ^ (u64::from(a.id).wrapping_mul(0x2545_F491_4F6C_DD1D));
            x ^= x >> 33;
            let drawn = (x % names.len() as u64) as usize;
            let resolved: Vec<String> = names.iter().map(|k| resolve(k)).collect();
            let index = (0..names.len()).map(|i| (drawn + i) % names.len()).find(|i| !used.contains(&resolved[*i])).unwrap_or(drawn);
            used.insert(resolved[index].clone());
            self.actors.insert(a.id, resolved[index].clone());
        }
    }
}

/// The `[Text]` fields of a map's TGI (lower-case keys).
fn map_table(data: &DataRoot, path: &str) -> HashMap<String, String> {
    let tgi = match path.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.TGI"),
        None => format!("{path}.TGI"),
    };
    let mut out = HashMap::new();
    for n in datafile::load(data, &tgi).iter().filter(|n| n.name.eq_ignore_ascii_case("Text")) {
        for (k, v) in &n.fields {
            out.insert(k.to_ascii_lowercase(), v.clone());
        }
    }
    out
}

type Lists = (HashMap<String, Vec<String>>, HashMap<String, String>);

static CACHE: std::sync::OnceLock<Lists> = std::sync::OnceLock::new();
static WARM: std::sync::Mutex<Option<std::thread::JoinHandle<Lists>>> = std::sync::Mutex::new(None);

/// Parse the name lists in the background from the start (every Buildings
/// file is read: ~0.3 s that would stall the first battle frame).
pub fn warm(root: DataRoot) {
    if let Ok(mut g) = WARM.lock() {
        if g.is_none() && CACHE.get().is_none() {
            *g = Some(std::thread::spawn(move || parse_lists(&root)));
        }
    }
}

/// The NameLists (ids -> name keys) and every Thing's `name_list_ids`
/// (Thing IDS -> list ids), parsed once.
fn name_lists(data: &DataRoot) -> Lists {
    CACHE
        .get_or_init(|| {
            let warmed = WARM.lock().ok().and_then(|mut g| g.take()).and_then(|h| h.join().ok());
            warmed.unwrap_or_else(|| parse_lists(data))
        })
        .clone()
}

fn parse_lists(data: &DataRoot) -> Lists {
    {
        {
            let mut lists = HashMap::new();
            for f in super::gamedata::files_with(data, "Namelists", ".tgi", false) {
                for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("NameList")) {
                    let Some(ids) = n.get("IDS").or(n.get("ids")) else { continue };
                    let names: Vec<String> = n.children_named("Name").filter_map(|c| c.get("name")).map(|s| s.trim_matches('"').to_string()).collect();
                    lists.insert(ids.to_ascii_lowercase(), names);
                }
            }
            let mut centers = HashMap::new();
            for f in super::gamedata::files_with(data, "Buildings", ".tgi", true) {
                for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Thing")) {
                    if let (Some(ids), Some(list)) = (n.get("IDS").or(n.get("ids")), n.get("name_list_ids")) {
                        if !list.eq_ignore_ascii_case("null") {
                            centers.insert(ids.to_ascii_lowercase(), list.to_ascii_lowercase());
                        }
                    }
                }
            }
            (lists, centers)
        }
    }
}

/// city_name_vfx (Effects/text_effects.tgi): rise_to_height and font.
const NAME_RISE: f32 = 10.0;
const NAME_FONT: &str = "large_outline";

/// The floating city names of this frame, drawn under the HUD (the
/// commands go first in the UI draw list).
#[allow(clippy::too_many_arguments)]
pub fn place(
    mut front: bevy::prelude::ResMut<super::front::Front>,
    battle: Option<bevy::prelude::Res<Battle>>,
    mut draw: bevy::prelude::ResMut<super::render::UiDraw>,
    windows: bevy::prelude::Query<&bevy::window::Window, bevy::prelude::With<bevy::window::PrimaryWindow>>,
    cameras: bevy::prelude::Query<(&bevy::prelude::Camera, &bevy::prelude::GlobalTransform), bevy::prelude::With<crate::camera::NativeCamera>>,
) {
    use bevy::prelude::Vec3;
    let Some(b) = battle else { return };
    let f = front.as_mut();
    if f.page != super::front::Page::Game {
        return;
    }
    if !f.ui.uvars.get_bool("ViewShowVFXObjectName") {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((camera, tf)) = cameras.single() else { return };
    let scale = window.scale_factor();
    let Some(hud) = f.hud.as_ref() else { return };
    let w = &b.world;
    let format = f.ui.strings.resolve("#template_vfx_VFXObjectName_format|s");
    let mut spots: Vec<(String, f32, f32)> = Vec::new();
    for (id, name) in &hud.names.actors {
        let Some(a) = w.actor(*id).filter(|a| a.alive && w.data.def(a.def).city) else { continue };
        if !super::fog::drawn(&b, *id) {
            continue;
        }
        let ground = w.ground(a.pos.truncate()).z;
        let p = crate::battle_view::to_render(Vec3::new(a.pos.x, a.pos.y, ground + NAME_RISE));
        let Ok(s) = camera.world_to_viewport(tf, p) else { continue };
        let (x, y) = (s.x * scale, s.y * scale);
        if x < -200.0 || y < -50.0 || x > window.physical_width() as f32 + 200.0 || y > window.physical_height() as f32 + 50.0 {
            continue;
        }
        spots.push((format.replacen("%s", name, 1), x, y));
    }
    if spots.is_empty() {
        return;
    }
    let mut cmds = Vec::new();
    for (text, x, y) in spots {
        // the text's top half a line above the point (a tall area: the
        // label rebuild clips glyphs to it)
        let rect = aa_ui::layout::Rect::new(x - 300.0, y - 30.0, x + 300.0, y + 90.0);
        let spec = aa_ui::ui::LabelSpec {
            font: NAME_FONT,
            rect,
            clip: [0.0; 4],
            justify: aa_ui::ve::Justify::Center,
            multiline: false,
            color: [1.0, 1.0, 1.0, 1.0],
            background: [0.0; 4],
            scroll: 0.0,
        };
        let drawn = f.ui.label_rebuild(&text, &spec);
        let fonts = f.ui.ui_fonts();
        let mut faces: Vec<aa_ui::draw::TextFace> = Vec::new();
        let mut glyphs = Vec::new();
        for q in &drawn.quads {
            let fo = fonts.font(q.font);
            let ids = fo.map(|x| x.ids.clone()).unwrap_or_default();
            let face = match faces.iter().position(|x| x.ids == ids) {
                Some(i) => i,
                None => {
                    faces.push(aa_ui::draw::TextFace { ids, face: fo.map(|x| x.face.clone()) });
                    faces.len() - 1
                }
            };
            glyphs.push(aa_ui::draw::GlyphQuad {
                c: q.glyph,
                face,
                rect: aa_ui::layout::Rect::new(q.geo[0], q.geo[1], q.geo[2], q.geo[3]),
                cell: [q.geo[4], q.geo[5]],
                color: q.color,
                highlight: q.highlight,
                image: fo.and_then(|x| x.images.get(&q.glyph)).map(|i| i.path.clone()),
            });
        }
        cmds.push(aa_ui::draw::DrawCmd {
            widget: "CityNames".into(),
            ve: "city_name_vfx".into(),
            kind: aa_ui::draw::DrawKind::Text(aa_ui::draw::Text {
                text,
                face: f.ui.font_face(NAME_FONT),
                font: NAME_FONT.into(),
                faces,
                glyphs,
                rect,
                justify: aa_ui::ve::Justify::Center,
                multiline: false,
                color: [1.0, 1.0, 1.0, 1.0],
                clip: None,
            }),
        });
    }
    let rest = std::mem::take(&mut draw.list.commands);
    cmds.extend(rest);
    draw.list.commands = cmds;
}

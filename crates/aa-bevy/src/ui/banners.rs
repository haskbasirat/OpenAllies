//! Organization banners, from the original data and art:
//! * every organization (regiment, HQ organization) shows its banner, the
//!   organization's own view model (template_organization.tgi [View]
//!   `banner_ids` / `banner_icon`, `z_offset` 5 for AAOrganization),
//!   Organizations/Banners/Shared/Banner.nif: a 2.41 m camera-facing
//!   billboard (NiBillboardNode mode 0, zMode11) of two planes,
//!   `PlayerColorPlane` (BannerPlayerColor.png in the kingdom colour, the
//!   EmRC emissive replacement) and `DecalPlane` (alpha test 130), drawn at
//!   the organization's position (OrgCombat +20/+24, it follows the leader)
//!   raised by `z_offset` over the ground;
//! * the decal plane layers template_banner_aa.tgi's AABanner decals in list
//!   order: the nation base frame `%sBannerBase.png` (Banner resource_prefix),
//!   rout (flashing), out of supply, upkeep shortage (flashing), entrenched,
//!   the banner icon, engaged, the experience rank (BannerRankBase /
//!   BannerRank1..3 by level) and the HQ badge (BADGE: the division
//!   insignia of the organization's HQ, Nations/<nation>.tgi [Badge]);
//! * a selected organization's view shows the VFXMoraleBar
//!   (Effects/MoraleBar/MoraleBar.NIF) above its banner;
//! * a focused organization (cursor over the banner or a member, or
//!   selected itself or through its structure) shows
//!   `banner_focus_vfx` (Effects/Selection/BannerFocus.NIF, BannerFocusBase)
//!   tinted with the diplomatic colour (0x454DC0: self green, friend blue,
//!   neutral grey 0.8, enemy red).
//!
//! Size: the banner, its focus frame and morale bar keep a constant screen
//! size, their model size at the default zoom's camera distance (read from
//! hd-v10-original-hq.png: zoomed in 1.6x over the default, the banner is
//! the same fraction of the screen height as at the default zoom, while the
//! soldiers are 1.6x larger).
//!
//! Compare against the original: the badge each HQ gets (here: per
//! kingdom, HQ organizations in creation order take the nation's badges in
//! list order), the banner raise / fade AVars (BannerRaiseHeight 2,
//! BannerRaiseSpeed 0.5, BannerFadeDurationMS 1000) are not modelled, and
//! the morale bar's height over the banner is read from
//! hd-v10-original-hq.png (1.7 m over the banner centre).
use super::datafile::{self, Node};
use super::nifart;
use crate::battle_view::{ActorView, to_render};
use crate::camera::NativeCamera;
use crate::input::Selecting;
use crate::session::Battle;
use aa_play::Unit;
use bevy::prelude::*;
use std::collections::HashMap;

/// The flashing decals' toggle (AVar UIFlashToggleSeconds).
const FLASH_SECONDS: f32 = 0.5;
/// The morale bar's centre over the banner centre (metres, read from the
/// original screenshot).
const MORALE_OVER_BANNER: f32 = 1.7;
/// The camera distance of the default zoom (camera: 100 / sin 40 deg at
/// zoom 1.0): banners keep the screen size they have there.
const REFERENCE_DISTANCE: f32 = 155.57;

#[derive(Component)]
pub struct BannerEntity;

/// A thing's banner fields ([View], last definition wins).
#[derive(Clone, Debug, Default)]
struct ThingBanner {
    banner_ids: Option<String>,
    icon: Option<String>,
    z_offset: f32,
}

/// The layers one banner image is composed of.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Key {
    color: [u8; 3],
    layers: Vec<String>,
}

#[derive(Resource, Default)]
pub struct BannerArt {
    /// The banner material of the object display's organization.
    pub selected: Option<Handle<StandardMaterial>>,
    loaded: bool,
    quad: Handle<Mesh>,
    size: f32,
    things: HashMap<String, ThingBanner>,
    /// banner ids -> resource prefix (Organizations/Banners/*.tgi).
    banners: HashMap<String, String>,
    /// nation short id -> badge texture paths (data paths).
    badges: HashMap<String, Vec<String>>,
    layers: HashMap<String, Option<image::RgbaImage>>,
    composed: HashMap<Key, Handle<StandardMaterial>>,
    focus: Vec<Handle<StandardMaterial>>,
    focus_size: f32,
    morale: Vec<Handle<StandardMaterial>>,
    morale_size: Vec2,
    pool: Vec<Entity>,
    /// Banners drawn this frame: (organization, render centre, half size),
    /// for picking.
    pub placed: Vec<(u32, Vec3, f32)>,
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// Template chain fields and children, then the node's own (as
/// gamedata::GameData::resolve).
fn resolve(templates: &HashMap<String, Node>, n: &Node, depth: u32) -> Node {
    let parent = n.attrs.get("template").or(n.attrs.get("inherit")).or(n.attrs.get("Template")).or(n.attrs.get("Inherit")).map(|s| lower(s));
    let mut out = match parent.and_then(|p| templates.get(&p)).filter(|_| depth < 24) {
        Some(t) => resolve(templates, t, depth + 1),
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
        out.children.push(resolve(templates, c, depth + 1));
    }
    out
}

/// A field of the merged [View] children (later definitions win).
fn view_field(n: &Node, key: &str) -> Option<String> {
    n.children_named("View").filter_map(|v| v.get(key)).last().map(str::to_string)
}

fn data_path(p: &str) -> String {
    p.trim().trim_start_matches(['/', '\\']).replace('\\', "/")
}

impl BannerArt {
    fn load(&mut self, front: &super::front::Front, images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>, meshes: &mut Assets<Mesh>) {
        self.loaded = true;
        self.quad = meshes.add(Rectangle::new(1.0, 1.0));
        let data = &front.ui.store.root;
        // templates (Templates/ and the regiment files' own) and things
        let mut files = super::gamedata::files_with(data, "Templates", ".tgi", false);
        files.extend(super::gamedata::files_with(data, "Regiments", ".tgi", true));
        let mut templates: HashMap<String, Node> = HashMap::new();
        let mut things: Vec<Node> = Vec::new();
        for f in &files {
            for n in datafile::load(data, f) {
                if n.name.eq_ignore_ascii_case("Template") {
                    if let Some(name) = n.args.first() {
                        templates.insert(lower(name), n.clone());
                    }
                } else if n.name.eq_ignore_ascii_case("Thing") {
                    things.push(n);
                }
            }
        }
        for t in &things {
            let r = resolve(&templates, t, 0);
            let Some(ids) = r.get("IDS") else { continue };
            let icon = view_field(&r, "banner_icon").filter(|s| !s.eq_ignore_ascii_case("null") && !s.contains('%')).map(|s| data_path(&s));
            self.things.insert(
                lower(ids),
                ThingBanner {
                    banner_ids: view_field(&r, "banner_ids").map(|s| lower(&s)),
                    icon,
                    z_offset: view_field(&r, "z_offset").and_then(|s| s.trim().parse().ok()).unwrap_or(0.0),
                },
            );
        }
        for f in super::gamedata::files_with(data, "Organizations/Banners", ".tgi", false) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Banner")) {
                if let (Some(ids), Some(prefix)) = (n.get("ids").or(n.get("IDS")), n.get("resource_prefix")) {
                    self.banners.insert(lower(ids), prefix.to_string());
                }
            }
        }
        for f in super::gamedata::files_with(data, "Nations", ".tgi", false) {
            for n in datafile::load(data, &f).iter().filter(|n| n.name.eq_ignore_ascii_case("Nation")) {
                let ids = lower(n.get("IDS").unwrap_or_default());
                let short = ids.strip_prefix("nation_").unwrap_or(&ids).to_string();
                let list: Vec<String> = n.children_named("Badge").filter_map(|b| b.get("badge_texture")).map(|p| format!("Nations/{}", data_path(p))).collect();
                self.badges.insert(short, list);
            }
        }
        // model sizes
        self.size = data.read("Organizations/Banners/Shared/Banner.nif").ok().and_then(|b| nifart::extent(&b)).map(|(w, _)| w).unwrap_or(2.41);
        if let Ok(bytes) = data.read("Effects/Selection/BannerFocus.NIF") {
            self.focus_size = nifart::extent(&bytes).map(|(w, _)| w).unwrap_or(self.size);
        }
        if let Some(img) = self.layer(data, "Effects/Selection/BannerFocusBase.tga") {
            for c in relation_colors() {
                let mut i = super::tex::make_image(img.width(), img.height(), img.as_raw().clone(), super::tex::Variant::default(), false);
                i.sampler = bevy::image::ImageSampler::linear();
                self.focus.push(materials.add(StandardMaterial {
                    base_color: Color::srgb(c[0], c[1], c[2]),
                    base_color_texture: Some(images.add(i)),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(130.0 / 255.0),
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }));
            }
        }
        if let Ok(bytes) = data.read("Effects/MoraleBar/MoraleBar.NIF") {
            self.morale_size = nifart::extent(&bytes).map(|(w, h)| Vec2::new(w, h)).unwrap_or(Vec2::new(1.12, 0.16));
            for f in nifart::flip_frames(&bytes) {
                self.morale.push(super::selection::material(images, materials, f, Color::WHITE));
            }
        }
        for k in ["regiment_rus_infantry", "regiment_ger_infantry", "regiment_rus_hq_infantry", "regiment_usa_hq_infantry_unpacked"] {
            debug!("banners: {k} {:?}", self.things.get(k));
        }
        debug!(
            "banners: {} things, {} banners, size {}, focus {}, morale {} frames {}",
            self.things.len(),
            self.banners.len(),
            self.size,
            self.focus_size,
            self.morale.len(),
            self.morale_size
        );
    }

    fn layer(&mut self, data: &aa_ui::fs::DataRoot, path: &str) -> Option<image::RgbaImage> {
        if let Some(l) = self.layers.get(path) {
            return l.clone();
        }
        // TGA has no signature: by extension
        let tga = path.to_ascii_lowercase().ends_with(".tga");
        let img = data
            .read(path)
            .ok()
            .and_then(|b| if tga { image::load_from_memory_with_format(&b, image::ImageFormat::Tga).ok() } else { image::load_from_memory(&b).ok() })
            .map(|i| {
            let i = i.to_rgba8();
            if i.width() != 64 || i.height() != 64 { image::imageops::resize(&i, 64, 64, image::imageops::FilterType::Triangle) } else { i }
        });
        if img.is_none() {
            debug!("banners: no texture {path}");
        }
        self.layers.insert(path.to_string(), img.clone());
        img
    }

    /// The composed banner material: the player colour plane, then the
    /// decal plane's layers (alpha test 128 / 130, no blending).
    fn material(&mut self, data: &aa_ui::fs::DataRoot, key: &Key, images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        if let Some(m) = self.composed.get(key) {
            return m.clone();
        }
        debug!("banners: compose {:?} {:?}", key.color, key.layers);
        let mut out = vec![0u8; 64 * 64 * 4];
        if let Some(pc) = self.layer(data, "Organizations/Banners/Shared/BannerPlayerColor.png") {
            for (i, p) in pc.pixels().enumerate() {
                if p[3] >= 128 {
                    for c in 0..3 {
                        out[i * 4 + c] = ((p[c] as u32 * key.color[c] as u32) / 255) as u8;
                    }
                    out[i * 4 + 3] = 255;
                }
            }
        }
        for l in &key.layers {
            let Some(img) = self.layer(data, l) else { continue };
            for (i, p) in img.pixels().enumerate() {
                if p[3] > 130 {
                    out[i * 4..i * 4 + 3].copy_from_slice(&p.0[..3]);
                    out[i * 4 + 3] = 255;
                }
            }
        }
        let mut img = super::tex::make_image(64, 64, out, super::tex::Variant::default(), false);
        img.sampler = bevy::image::ImageSampler::linear();
        let m = materials.add(StandardMaterial {
            base_color_texture: Some(images.add(img)),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        self.composed.insert(key.clone(), m.clone());
        m
    }
}

/// The diplomatic colours of 0x454DC0 by relation (0 self, 1 friend,
/// 2 neutral, 3 enemy).
pub fn relation_colors() -> [[f32; 3]; 4] {
    [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.8, 0.8, 0.8], [1.0, 0.0, 0.0]]
}

/// The HQ badge of every HQ organization: per kingdom, in creation order,
/// the nation's badges in list order.
fn hq_badges(w: &aa_play::World, art: &BannerArt) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let mut next: HashMap<u32, usize> = HashMap::new();
    for o in &w.orgs {
        if !w.data.def(o.def).has_component("HQComponent") {
            continue;
        }
        let Some(k) = w.kingdoms.iter().find(|k| k.id == o.kingdom) else { continue };
        let Some(list) = art.badges.get(&lower(&k.nation)).filter(|l| !l.is_empty()) else { continue };
        let i = next.entry(o.kingdom).or_default();
        out.insert(o.id, list[*i % list.len()].clone());
        *i += 1;
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut art: ResMut<BannerArt>,
    front: Res<super::front::Front>,
    battle: Option<Res<Battle>>,
    sel: Res<Selecting>,
    time: Res<Time>,
    cameras: Query<&GlobalTransform, With<NativeCamera>>,
    views: Query<(&ActorView, &GlobalTransform)>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut entities: Query<(&mut Transform, &mut MeshMaterial3d<StandardMaterial>, &mut Visibility), With<BannerEntity>>,
) {
    let Some(b) = battle else { return };
    if !art.loaded {
        art.load(&front, &mut images, &mut materials, &mut meshes);
    }
    let Ok(cam) = cameras.single() else { return };
    let cam_rot = cam.compute_transform().rotation;
    let up = cam_rot * Vec3::Y;
    let toward = cam_rot * Vec3::Z;
    let eye = cam.translation();
    let w = &b.world;
    let data = &front.ui.store.root;
    let view_pos: HashMap<u32, Vec3> = views.iter().map(|(v, tf)| (v.id, tf.translation())).collect();
    let badges = hq_badges(w, &art);
    let flash_on = (time.elapsed_secs() / FLASH_SECONDS) as u64 % 2 == 0;
    // the focused organizations (0x458ED0: the focus VFX runs for an
    // object flagged hovered or selected, 0x8000 / 0x10000, or whose
    // organization's is; the selection VFX of organizations is
    // /special/empty.NIF): the hovered member's, the selected ones and the
    // organization of a selected structure (the original's selected HQ
    // shows its banner's green corners, original-ui/06-corps.png)
    let selected: Vec<u32> = b.selection.iter().filter_map(|u| match u {
        Unit::Org(o) => Some(*o),
        _ => None,
    }).collect();
    let mut focused: Vec<u32> = sel.hover.and_then(|id| w.actor(id)).and_then(|a| a.org).into_iter().collect();
    for u in &b.selection {
        match u {
            Unit::Org(o) => focused.push(*o),
            Unit::Actor(a) => focused.extend(w.actor(*a).and_then(|x| x.org)),
        }
    }
    let mut want: Vec<(Handle<StandardMaterial>, Transform)> = Vec::new();
    let mut placed = Vec::new();
    // the object display's banner: the first selected organization's, or
    // the HQ organization of a selected HQ structure
    let shown_org = match b.selection.first() {
        Some(Unit::Org(o)) => Some(*o),
        Some(Unit::Actor(a)) => w.actor(*a).and_then(|x| x.org),
        None => None,
    };
    art.selected = None;
    // an organization the local kingdom does not see has no banner
    for o in w.orgs.iter().filter(|o| o.alive && aa_play::fog::org_visible(w, o.id, Some(b.local))) {
        let def = w.data.def(o.def);
        let Some(info) = art.things.get(&lower(&def.id)).cloned() else { continue };
        let Some(k) = w.kingdoms.iter().find(|k| k.id == o.kingdom) else { continue };
        let nation = lower(&k.nation);
        // banner ids: the thing's own, else the nation's (use_faction_banner)
        let prefix = info
            .banner_ids
            .as_ref()
            .and_then(|ids| art.banners.get(ids))
            .or_else(|| art.banners.get(&format!("{nation}_banner")))
            .cloned();
        let Some(prefix) = prefix else { continue };
        if prefix.to_ascii_lowercase().starts_with("ambient") {
            continue;
        }
        // the organization position (follows the leader), smoothed by the
        // members' drawn positions
        let members: Vec<Vec3> = o.members.iter().filter(|m| w.actor(**m).is_some_and(|a| a.alive)).filter_map(|m| view_pos.get(m).copied()).collect();
        if members.is_empty() {
            continue;
        }
        let combat = aa_play::combat::org_state(w, o.id);
        let native = combat.map(|c| c.position).filter(|p| p.length_squared() > 0.0);
        let centre = match native {
            Some(p) => {
                // the combat position is per tick; the leader's drawn
                // position keeps it smooth between ticks
                let leader = o.members.iter().filter_map(|m| w.actor(*m).filter(|a| a.alive).map(|a| (m, a.pos.truncate()))).min_by(|x, y| x.1.distance(p).total_cmp(&y.1.distance(p)));
                match leader.and_then(|(m, lp)| view_pos.get(m).map(|v| (v, lp))) {
                    Some((v, lp)) if lp.distance(p) < 0.5 => Vec2::new(v.x, -v.z),
                    _ => p,
                }
            }
            None => {
                let c = members.iter().copied().sum::<Vec3>() / members.len() as f32;
                Vec2::new(c.x, -c.z)
            }
        };
        let ground = w.ground(centre);
        let pos = to_render(Vec3::new(centre.x, centre.y, ground.z + info.z_offset));
        // layers in AABanner decal order
        let mut layers = vec![format!("Organizations/Banners/{prefix}BannerBase.png")];
        let routing = combat.is_some_and(|c| c.routing.is_some());
        if routing && flash_on {
            layers.push("Organizations/Banners/Shared/BannerRout.png".into());
        }
        if w.books.org(o.id).is_some_and(|s| s.out_of_supply) {
            layers.push("Organizations/Banners/Shared/BannerOutOfSupply.png".into());
        }
        if w.books.kingdom(o.kingdom).is_some_and(|s| s.shortage_ratio > 0.0) && flash_on {
            layers.push("Organizations/Banners/Shared/BannerBleeding.png".into());
        }
        if combat.is_some_and(|c| c.entrenched) {
            layers.push("Organizations/Banners/Shared/BannerEntrenched.png".into());
        }
        if let Some(icon) = &info.icon {
            layers.push(icon.clone());
        }
        if combat.is_some_and(|c| c.engaging()) {
            layers.push("Organizations/Banners/Shared/BannerEngaged1.png".into());
        }
        let rank = ["BannerRankBase", "BannerRank1", "BannerRank2", "BannerRank3"];
        let level = combat.map(|c| c.level).unwrap_or(0).min(rank.len() - 1);
        if level > 0 {
            layers.push(format!("Organizations/Banners/Shared/{}.png", rank[level]));
        }
        // the HQ badge: the HQ organization's own, else its HQ's
        let hq_org = o.hq.and_then(|h| w.actor(h)).and_then(|a| a.org);
        if let Some(badge) = badges.get(&o.id).or(hq_org.and_then(|h| badges.get(&h))) {
            layers.push(badge.clone());
        }
        let key = Key { color: k.color, layers };
        let m = art.material(data, &key, &mut images, &mut materials);
        if shown_org == Some(o.id) {
            art.selected = Some(m.clone());
        }
        // constant screen size (banner_focus_vfx constant_screen_size; the
        // original banners keep their screen size over the zoom range, see
        // the module notes): the model size at the default zoom's distance
        let k = (pos - eye).length() / REFERENCE_DISTANCE;
        let size = art.size * k;
        want.push((m, Transform::from_translation(pos).with_rotation(cam_rot).with_scale(Vec3::new(size, size, 1.0))));
        placed.push((o.id, pos, size * 0.5));
        let relation = aa_play::setup::relation(w, Some(b.local), Some(o.kingdom)) as usize;
        if focused.contains(&o.id) {
            if let Some(f) = art.focus.get(relation.min(3)).cloned() {
                let s = art.focus_size * k;
                want.push((f, Transform::from_translation(pos + toward * 0.05 * k).with_rotation(cam_rot).with_scale(Vec3::new(s, s, 1.0))));
            }
        }
        if selected.contains(&o.id) && !art.morale.is_empty() {
            let frames = &art.morale;
            let i = ((1.0 - o.morale.clamp(0.0, 1.0)) * (frames.len() - 1) as f32).round() as usize;
            let s = art.morale_size * k;
            want.push((frames[i.min(frames.len() - 1)].clone(), Transform::from_translation(pos + up * MORALE_OVER_BANNER * k).with_rotation(cam_rot).with_scale(s.extend(1.0))));
        }
    }
    art.placed = placed;
    while art.pool.len() < want.len() {
        let e = commands
            .spawn((
                Mesh3d(art.quad.clone()),
                MeshMaterial3d(want[art.pool.len()].0.clone()),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                BannerEntity,
            ))
            .id();
        art.pool.push(e);
    }
    for (i, e) in art.pool.iter().enumerate() {
        let Ok((mut tf, mut mat, mut vis)) = entities.get_mut(*e) else { continue };
        match want.get(i) {
            Some((m, t)) => {
                *tf = *t;
                if mat.0 != *m {
                    mat.0 = m.clone();
                }
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

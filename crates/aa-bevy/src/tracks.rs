//! Ground tracks: tire tracks, tread tracks and footprints, ported from the
//! original's view-side Track objects and its track decal list.
//!
//! Data (ActorData::LoadData 0x464BD0, MoverComponent `TrackInfo` children,
//! template_tracks.tgi): track_type DISCRETE (footprints) or CONTINUOUS
//! (vehicles), track_texture (resource_prefix formatted, 4763C0 relative),
//! track_texture_mirrored, track_placement_pcg, track_offset, track_size and
//! fade_sec.
//!
//! Per view actor (VActor Init 0x451F90 builds one 0x74-byte Track per
//! TrackInfo with 0x42D7B0 and 0x42D830; this+0x144 is the array):
//! * VActor SetPosition 0x453A50 runs the tracks (0x451700) when the actor is
//!   set, the actor has tracks and the option RenderDrawFootsteps (UVar 107,
//!   [B39940]+0x1AC) is on, BEFORE the old position (+0xD4) is replaced; so
//!   every call sees the new position and the one of the previous call.
//! * 0x42E290: a CONTINUOUS track runs 0x42E030, a DISCRETE one 0x42DC70.
//! * 0x42E030 lays the strip: the segments to every position-history sample
//!   (Actor+0x144, recorded every TickCountPerTrackPlacement ticks by 0x50A970
//!   from 0x4D0EE0) newer than the last call and older than now, then one to
//!   the current position; each segment is one quad (0x42D9B0) whose texture
//!   v runs on by the distance over the track's length.
//! * 0x42DC70 puts a footprint down each time the walked distance, counted in
//!   move-animation cycles (walk_distance of the move 0x45DA00 picks for the
//!   speed), passes a whole cycle, starting at track_placement_pcg.
//!
//! The quads (0x7196F0, 0x54 bytes) go to the track list ([B36518]+0x144,
//! 0x725610) and are dropped once their fade end has passed (0x7198A0). The
//! list is drawn every frame (0x719B40 -> 0x7198C0): full opacity for the
//! first half of fade_sec, then a straight fade to nothing at the end;
//! texture times the vertex colour (stage states of 0x73D7F0 for the track
//! group: modulate colour and alpha, source-alpha blending, no depth write,
//! TrackDepthBias -0.00001 and TrackSlopeScaleBias 0 from AVars.tgi).
//!
//! Stand-ins, named where they are used: the history's starting phase, the
//! terrain light the vertex colour takes (0x71C580), the animation state and
//! set the footprints read, and the depth bias.
use crate::battle_view::ActorView;
use crate::session::{AppState, Battle};
use aa_play::{Activity, ActorId};
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::native::TypedValue;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const PC: aa_numeric::Precision = aa_numeric::Precision::Double53;
/// Bump when the extracted layout changes.
const CACHE_VERSION: &str = "aa-bevy-tracks-1";

/// SVars.tgi `TickCountPerTrackPlacement` (SVar 10, Data/Game/SVars.tgi: 10).
const TICK_COUNT_PER_TRACK_PLACEMENT: i32 = 10;

/// Stand-in for TrackDepthBias (-0.00001 in AVars.tgi, slope scale 0): the
/// quads lie at the actor's height, lifted this much (render metres), like
/// the projected decals (effects/decal.rs LIFT).
const LIFT: f32 = 0.015;

/// Where the batches sit along the view for Bevy's transparent sort: just
/// beyond the unlit decals (2000 m, effects/mod.rs), so the tracks draw
/// before them; the lit ground planes (3000 m) draw first.
const SORT_AHEAD: f32 = 2001.0;

pub struct TracksPlugin;

impl Plugin for TracksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tracks>()
            .add_systems(Update, update.after(crate::battle_view::sync_actors).run_if(in_state(AppState::Battle)))
            .add_systems(OnExit(AppState::Battle), leave);
    }
}

/// One TrackInfo (ActorData+0x25C entry, 0x24 bytes).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackDef {
    /// +00: CONTINUOUS (1) or DISCRETE (0).
    pub continuous: bool,
    /// +04 track_texture_mirrored.
    pub mirrored: bool,
    /// +08 track_placement_pcg.
    pub placement_pcg: f32,
    /// +0C track_texture, Data-relative.
    pub texture: String,
    /// +10/+14 track_offset (across, along).
    pub offset: [f32; 2],
    /// +18/+1C track_size (width, length).
    pub size: [f32; 2],
    /// +20 fade_sec.
    pub fade_sec: f32,
}

#[derive(Default, Serialize, Deserialize)]
struct TrackTable {
    source_key: String,
    /// Actor IDS (lower case) -> its TrackInfo list.
    actors: BTreeMap<String, Vec<TrackDef>>,
    notes: Vec<String>,
}

fn value(node: &ResolvedNode<'_>, name: &str) -> Option<TypedValue> {
    node.field(name, PC).ok().flatten()
}

fn num(node: &ResolvedNode<'_>, name: &str) -> Option<f32> {
    match value(node, name)? {
        TypedValue::Scalar(s) => Some(f32::from_bits(s.bits)),
        TypedValue::Integer { value, .. } => Some(value as f32),
        TypedValue::String { value, .. } => {
            let v = value.trim();
            match v.strip_suffix('%') {
                Some(p) => p.trim().parse::<f32>().ok().map(|x| x / 100.0),
                None => v.parse().ok(),
            }
        }
        _ => None,
    }
}

fn point2(node: &ResolvedNode<'_>, name: &str) -> Option<[f32; 2]> {
    match value(node, name)? {
        TypedValue::Point { bits, .. } if bits.len() >= 2 => Some([f32::from_bits(bits[0]), f32::from_bits(bits[1])]),
        _ => None,
    }
}

/// A string field and the folder of the file that assigned it.
fn string(node: &ResolvedNode<'_>, name: &str) -> Option<(String, String)> {
    match value(node, name)? {
        TypedValue::String { value, source, .. } => {
            let path = source.path.replace('\\', "/");
            let folder = path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
            Some((value.trim().to_owned(), folder))
        }
        _ => None,
    }
}

/// 0x464BD0 465DA4..465F74 for one actor: its TrackInfo list (an actor
/// without a MoverComponent has none). An unknown track_type throws in the
/// original ("Unknown track type '%s' on object '%s'"); here the actor gets
/// no tracks and a note.
fn actor_tracks(catalog: &ActorCatalog, id: &str, notes: &mut Vec<String>) -> Option<Vec<TrackDef>> {
    let actor = catalog.resolved_actor(id).ok()?;
    let Some(mover) = actor.child("MoverComponent").ok().flatten() else { return Some(Vec::new()) };
    // ActorData+B0: the View's resource_prefix (49DC40).
    let prefix = actor.child("View").ok().flatten().and_then(|v| string(&v, "resource_prefix")).map(|(p, _)| p).unwrap_or_default();
    let mut out = Vec::new();
    for t in mover.children("TrackInfo").unwrap_or_default() {
        let kind = string(&t, "track_type").map(|(k, _)| k).unwrap_or_default();
        let continuous = if kind.eq_ignore_ascii_case("CONTINUOUS") {
            true
        } else if kind.eq_ignore_ascii_case("DISCRETE") {
            false
        } else {
            notes.push(format!("Unknown track type '{kind}' on object '{id}'"));
            return Some(Vec::new());
        };
        let Some((format, folder)) = string(&t, "track_texture") else { continue };
        if format.is_empty() || format.eq_ignore_ascii_case("null") {
            notes.push(format!("{id}: a TrackInfo without a texture"));
            continue;
        }
        // 405D60: the format takes the resource prefix at its conversion.
        let expanded = if format.contains("%s") { format.replacen("%s", &prefix, 1) } else { format };
        out.push(TrackDef {
            continuous,
            mirrored: matches!(value(&t, "track_texture_mirrored"), Some(TypedValue::Boolean { value: true, .. })),
            placement_pcg: num(&t, "track_placement_pcg").unwrap_or(1.0),
            texture: crate::effects::data::relative(&folder, &expanded),
            offset: point2(&t, "track_offset").unwrap_or([0.0, 0.0]),
            size: point2(&t, "track_size").unwrap_or([0.333, 0.333]),
            fade_sec: num(&t, "fade_sec").unwrap_or(3.0),
        });
    }
    Some(out)
}

fn extract(catalog: &ActorCatalog, ids: &[String]) -> TrackTable {
    let mut t = TrackTable::default();
    for id in ids {
        let mut notes = Vec::new();
        match actor_tracks(catalog, id, &mut notes) {
            Some(list) if !list.is_empty() => {
                t.actors.insert(id.clone(), list);
            }
            Some(_) => {}
            None => notes.push(format!("actor {id}: not resolved")),
        }
        t.notes.extend(notes);
    }
    t
}

/// The table for these actors, from the cache when it matches, else from
/// the original catalog (seconds), then cached (like effects/data.rs).
fn load_table(ids: Vec<String>, folder: Option<PathBuf>) -> anyhow::Result<TrackTable> {
    let reconstruction = crate::effects::reconstruction_root().ok_or_else(|| anyhow::anyhow!("the reconstruction folder was not found"))?;
    let install = reconstruction.parent().ok_or_else(|| anyhow::anyhow!("install root"))?.to_path_buf();
    let mut ids: Vec<String> = ids.iter().map(|s| s.to_ascii_lowercase()).collect();
    ids.sort();
    ids.dedup();
    let mut cache = crate::effects::data::cache_dir(&install);
    let data_folder = folder.as_deref().map(aa_play::data::DataFolder::new).transpose()?;
    let key = match &data_folder {
        Some(f) => {
            cache = cache.join("folders").join(&f.key()[..16]);
            format!("{CACHE_VERSION}:{}:{}", f.key(), ids.len())
        }
        None => format!("{CACHE_VERSION}:{}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256, ids.len()),
    };
    let path = cache.join("tracks.json");
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(t) = serde_json::from_slice::<TrackTable>(&bytes)
        && t.source_key == key
    {
        return Ok(t);
    }
    let mut t = match &data_folder {
        Some(f) => extract(f.catalog(&install)?, &ids),
        None => {
            let mut sources = aa_play::data::Sources::open(&install)?;
            let catalog = ActorCatalog::load_v14(&mut sources.files).map_err(|e| anyhow::anyhow!("loading the original rules catalog: {e}"))?;
            extract(&catalog, &ids)
        }
    };
    t.source_key = key;
    let _ = std::fs::create_dir_all(&cache);
    if let Ok(bytes) = serde_json::to_vec(&t) {
        let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(t)
}

/// Native (x, y ground, z up) <- render (x, z, -y).
fn native(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// 0x42D750: normalise in place; a length of 1e-6 or less gives zero.
fn normalize_42d750(v: Vec3) -> Vec3 {
    let len = v.length();
    if len > 1.0e-6 { v * (1.0 / len) } else { Vec3::ZERO }
}

/// One Track (0x74 bytes; 0x42D7B0 construct, 0x42D830 initialise).
struct Track {
    def: Arc<TrackDef>,
    /// +28/+2C: half the size (0x42D830, times 0.5).
    half: Vec2,
    /// +38: the footprint cycle counter (-1: not walking).
    cycle: f32,
    /// +3C: the point the last segment was laid from.
    from: Vec3,
    /// +48: the centre of the last edge.
    center: Vec3,
    /// +54 / +60: the last edge's two ends. `fresh` stands for the
    /// constructor's equal ends (both the B43CF8 vector) that 0x42E030 tests.
    left: Vec3,
    right: Vec3,
    fresh: bool,
    /// +6C: texture v at the last edge (-1 at construction).
    v: f32,
    /// +70: the time of the last edge.
    time: f32,
}

impl Track {
    fn new(def: Arc<TrackDef>) -> Self {
        let half = Vec2::new(def.size[0] * 0.5, def.size[1] * 0.5);
        Track { def, half, cycle: -1.0, from: Vec3::ZERO, center: Vec3::ZERO, left: Vec3::ZERO, right: Vec3::ZERO, fresh: true, v: -1.0, time: 0.0 }
    }

    /// 0x42D8A0: the unit direction from `b` to `a` on the ground, the side
    /// (the direction turned left) and the centre: `a` (flag set) or `b`,
    /// moved by -offset.x along the side and offset.y along the direction.
    fn frame(&self, a: Vec3, b: Vec3, at_a: bool) -> (Vec3, Vec3, Vec3) {
        let dir = normalize_42d750(Vec3::new(a.x - b.x, a.y - b.y, 0.0));
        let side = Vec3::new(-dir.y, dir.x, 0.0);
        let base = if at_a { a } else { b };
        let across = -self.def.offset[0];
        let along = self.def.offset[1];
        let center = Vec3::new(across * side.x + base.x + along * dir.x, across * side.y + base.y + along * dir.y, across * 0.0 + base.z + along * dir.z);
        (side, center, dir)
    }

    /// 0x42DBA0: start the strip at `prev`, facing `cur`, without a quad.
    fn start(&mut self, cur: Vec3, prev: Vec3) {
        self.from = prev;
        let (side, center, _) = self.frame(cur, self.from, false);
        self.center = center;
        let s = side * self.half.x;
        self.left = center + s;
        self.right = center - s;
        self.fresh = false;
    }

    /// 0x42D9B0: the edge at `p` (facing away from `prev`) and, unless
    /// `skip`, the quad from the last edge to it.
    fn segment(&mut self, p: Vec3, prev: Vec3, time: f32, skip: bool, out: &mut Vec<Quad>) {
        self.from = prev;
        let (side, center, _) = self.frame(p, prev, true);
        let s = side * self.half.x;
        if self.v == -1.0 {
            self.v = 0.0;
        }
        let left = center + s;
        let right = center - s;
        let v = (center - self.center).length() / self.def.size[1] + self.v;
        if !skip {
            out.push(Quad::new(self.left, self.right, left, right, &self.def, self.v, v, time));
        }
        self.center = center;
        self.left = left;
        self.right = right;
        self.v = v;
        self.time = time;
    }

    /// 0x42E030, CONTINUOUS.
    fn continuous(&mut self, cur: Vec3, prev: Vec3, now: f32, history: Option<&History>, ground: &dyn Fn(Vec2) -> f32, out: &mut Vec<Quad>) {
        if self.fresh {
            self.start(cur, prev);
        }
        let mut from = self.from;
        if let Some(h) = history {
            // 0x50A9F0 places the cursor by +70 before the reset test.
            let since = self.time;
            if h.reset {
                self.start(cur, cur);
                self.v = 0.0;
                self.time = now;
                from = self.from;
            }
            // 0x50A9F0: from the first sample newer than +70, every sample
            // older than now (0x43D050 puts it on the ground).
            for (xy, t) in h.after(since) {
                if t >= now {
                    break;
                }
                let p = Vec3::new(xy.x, xy.y, ground(xy));
                self.segment(p, from, t, now - t >= self.def.fade_sec, out);
                from = p;
            }
        }
        if cur != prev {
            self.segment(cur, from, now, false, out);
        }
    }

    /// 0x42DC70, DISCRETE: `walking` is the animation state's type MOVE (2),
    /// `stride` the stride of 0x45DA00 (else the speed itself).
    fn discrete(&mut self, cur: Vec3, prev: Vec3, now: f32, walking: bool, speed: f32, stride: Option<f32>, out: &mut Vec<Quad>) {
        if !walking {
            self.cycle = -1.0;
            return;
        }
        let dist = (cur - prev).length();
        if dist == 0.0 || speed == 0.0 {
            return;
        }
        let cycles = dist / stride.unwrap_or(speed);
        if self.cycle == -1.0 {
            self.cycle = self.def.placement_pcg - cycles;
        }
        let total = cycles + self.cycle;
        self.cycle = total;
        if total < 1.0 {
            return;
        }
        let over = total - 1.0;
        self.cycle = over;
        // Back along this step by the part of a cycle already passed.
        let d = (cur - prev) * (over / cycles);
        let p = cur - d;
        let (side, center, dir) = self.frame(p, prev, true);
        let s = side * self.half.x;
        let f = dir * self.half.y;
        out.push(Quad::new(center + s - f, center - s - f, center + s + f, center - s + f, &self.def, 0.0, 1.0, now));
    }
}

/// One quad of the track list (0x7196F0): +00 a, +0C b, +18 c, +24 d in
/// drawing order, texture, mirrored, v at a/d and at b/c, the start time
/// and the fade end.
struct Quad {
    corners: [Vec3; 4],
    texture: String,
    mirrored: bool,
    v: [f32; 2],
    t0: f32,
    t1: f32,
}

impl Quad {
    /// The constructor's argument order: the first two ends (start of the
    /// quad), then the last two; +00 = first, +0C = third, +18 = fourth,
    /// +24 = second.
    #[allow(clippy::too_many_arguments)]
    fn new(first: Vec3, second: Vec3, third: Vec3, fourth: Vec3, def: &TrackDef, v0: f32, v1: f32, time: f32) -> Self {
        Quad { corners: [first, third, fourth, second], texture: def.texture.to_ascii_lowercase(), mirrored: def.mirrored, v: [v0, v1], t0: time, t1: time + def.fade_sec }
    }

    /// 0x7198C0: opaque for the first half of the fade, then linear to 0.
    fn alpha(&self, now: f32) -> f32 {
        let t = if self.t1 < now { self.t1 } else { now };
        let a = (1.0 - (t - self.t0) / (self.t1 - self.t0)) * 2.0;
        if a < 0.0 || a.is_nan() {
            0.0
        } else if a > 1.0 {
            1.0
        } else {
            a
        }
    }
}

/// The Actor+0x144 position history (0x50AB10, 0x50A970): a ring of
/// `period + 1` samples of the ground position and the world time.
struct History {
    samples: Vec<(Vec2, f32)>,
    read: usize,
    write: usize,
    reset: bool,
    last_frame: i32,
    last_xy: Option<Vec2>,
}

impl History {
    /// `max_fade` is the longest fade_sec of the CONTINUOUS tracks
    /// (0x4D74C0: floor(GameTicksPerGameSecond x fade + 0.5) /
    /// TickCountPerTrackPlacement + 1 is the period).
    fn new(max_fade: f32) -> Self {
        let ticks = (aa_play::TICKS_PER_SECOND as f32 * max_fade + 0.5).floor() as i32;
        let period = ticks / TICK_COUNT_PER_TRACK_PLACEMENT + 1;
        // Stand-in: the original starts last_frame at -(draw %
        // TickCountPerTrackPlacement), a World RNG draw at creation (the
        // 50AB10 draw aa-play makes at MOVER_COUNTDOWN and does not keep).
        History { samples: vec![(Vec2::ZERO, 0.0); (period + 1).max(1) as usize], read: 0, write: 0, reset: false, last_frame: 0, last_xy: None }
    }

    /// 0x4D0EE0's history step on a position change: 0x50A970 when the
    /// interval since the last sample has passed.
    fn moved(&mut self, frame: i32, time: f32, xy: Vec2) {
        if self.last_frame.wrapping_add(TICK_COUNT_PER_TRACK_PLACEMENT) > frame {
            return;
        }
        self.last_frame = frame;
        let n = self.samples.len();
        self.samples[self.write] = (xy, time);
        self.write = (self.write + 1) % n;
        if self.write == self.read {
            self.read = (self.read + 1) % n;
        }
    }

    /// The samples after the first one newer than `t` would be (0x50A9F0:
    /// the first sample whose time is above `t`), oldest first.
    fn after(&self, t: f32) -> impl Iterator<Item = (Vec2, f32)> + '_ {
        let n = self.samples.len();
        let count = (self.write + n - self.read) % n;
        (0..count).map(move |i| self.samples[(self.read + i) % n]).skip_while(move |s| s.1 <= t)
    }
}

/// One view actor's tracks and the position of the previous call (+0xD4).
struct ActorTracks {
    tracks: Vec<Track>,
    prev: Option<Vec3>,
    history: Option<History>,
}

struct Batch {
    entity: Entity,
    mesh: Handle<Mesh>,
}

#[derive(Resource, Default)]
struct Tracks {
    table: Option<Arc<TrackTable>>,
    loading: Option<Arc<Mutex<Option<anyhow::Result<TrackTable>>>>>,
    actors: HashMap<ActorId, ActorTracks>,
    quads: Vec<Quad>,
    batches: HashMap<String, Batch>,
    missing: Vec<String>,
    tick: u64,
}

fn leave(mut commands: Commands, mut tracks: ResMut<Tracks>) {
    for (_, b) in tracks.batches.drain() {
        commands.entity(b.entity).despawn();
    }
    *tracks = Tracks::default();
}

/// The track texture as a repeating image (D3D's default WRAP addressing).
fn texture_image(rel: &str) -> Option<Image> {
    let reconstruction = crate::effects::reconstruction_root()?;
    let path = crate::effects::bones::find_file(&reconstruction.join("assets/original/Data"), rel)?;
    let rgba = image::open(Path::new(&path)).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d { width, height, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba.into_raw(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..bevy::image::ImageSamplerDescriptor::default()
    });
    Some(image)
}

fn write_mesh(mesh: &mut Mesh, mut pos: Vec<[f32; 3]>, mut uvs: Vec<[f32; 2]>, mut colors: Vec<[f32; 4]>, mut indices: Vec<u32>) {
    if pos.is_empty() {
        for _ in 0..3 {
            pos.push([0.0, -1.0e4, 0.0]);
            uvs.push([0.0, 0.0]);
            colors.push([0.0; 4]);
        }
        indices = vec![0, 1, 2];
    }
    let normals = vec![[0.0, 1.0, 0.0]; pos.len()];
    *mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
}

/// The stride 0x45DA00 picks for `speed` among the set's move animations
/// (each takes the speeds up to the mean of its walk_distance and the next
/// one's, the last the rest; battle_view's Character::move_for). Stand-in:
/// the base set, not the stance's AspectAnimationSet (0x4D0720 -> 0x45E690).
fn stride_for(fx: Option<&crate::effects::data::ActorFx>, speed: f32) -> Option<f32> {
    let moves: Vec<f32> = fx?.view.animations.iter().filter(|a| a.kind == "MOVE").map(|a| a.walk_distance).collect();
    for (i, w) in moves.iter().enumerate() {
        let threshold = moves.get(i + 1).map_or(f32::MAX, |n| (w + n) * 0.5);
        if speed <= threshold {
            return Some(*w);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    front: Option<Res<crate::ui::front::Front>>,
    fx: Option<Res<crate::effects::FxData>>,
    mut tracks: ResMut<Tracks>,
    views: Query<(&ActorView, &Transform)>,
    cameras: Query<&Transform, (With<crate::camera::NativeCamera>, Without<ActorView>)>,
    mut batch_transforms: Query<&mut Transform, (Without<ActorView>, Without<crate::camera::NativeCamera>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<crate::look::OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(battle) = battle else { return };
    let tracks = &mut *tracks;
    if battle.is_added() {
        for (_, b) in tracks.batches.drain() {
            commands.entity(b.entity).despawn();
        }
        *tracks = Tracks::default();
    }
    let world = &battle.world;
    if tracks.table.is_none() && tracks.loading.is_none() {
        let ids: Vec<String> = world.data.defs.iter().map(|d| d.id.clone()).collect();
        let folder = crate::effects::bones::folder();
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        std::thread::spawn(move || {
            let r = load_table(ids, folder);
            *out.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        });
        tracks.loading = Some(slot);
    }
    if let Some(slot) = tracks.loading.clone() {
        let done = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
        match done {
            Some(Ok(t)) => {
                for n in t.notes.iter().take(8) {
                    warn!("tracks: {n}");
                }
                info!("tracks: TrackInfo of {} actors", t.actors.len());
                tracks.table = Some(Arc::new(t));
                tracks.loading = None;
            }
            Some(Err(e)) => {
                warn!("tracks: {e:#}");
                tracks.table = Some(Arc::new(TrackTable::default()));
                tracks.loading = None;
            }
            None => return,
        }
    }
    let Some(table) = tracks.table.clone() else { return };
    // The view's clock: the drawn time between the last two ticks.
    let alpha = crate::session::tick_fraction(&battle);
    let now = world.time - (1.0 - alpha) * aa_play::DT;
    let ground = |p: Vec2| world.map.height_at(p);

    // The history samples on each new tick (0x4D0EE0 on a position change).
    let new_tick = tracks.tick != world.tick;
    tracks.tick = world.tick;

    // UVar 107 RenderDrawFootsteps (default true) gates every track.
    let draw = front.as_ref().is_none_or(|f| f.ui.uvars.get_bool("RenderDrawFootsteps"));
    let mut made = Vec::new();
    for (view, transform) in &views {
        let Some(actor) = world.actors.get(view.id as usize).filter(|a| a.id == view.id) else { continue };
        let def = world.data.def(actor.def);
        let key = def.id.to_ascii_lowercase();
        let Some(list) = table.actors.get(&key) else { continue };
        let state = tracks.actors.entry(view.id).or_insert_with(|| {
            let max_fade = list.iter().filter(|t| t.continuous).map(|t| t.fade_sec).fold(None, |m: Option<f32>, f| Some(m.map_or(f, |m| m.max(f))));
            ActorTracks { tracks: list.iter().cloned().map(|d| Track::new(Arc::new(d))).collect(), prev: None, history: max_fade.map(History::new) }
        });
        if new_tick && let Some(h) = state.history.as_mut() {
            let xy = actor.pos.truncate();
            if h.last_xy.is_some_and(|l| l != xy) {
                h.moved(world.tick as i32, world.time, xy);
            }
            h.last_xy = Some(xy);
        }
        let cur = native(transform.translation);
        // The first call has no earlier position: it starts the strip here.
        let prev = state.prev.unwrap_or(cur);
        if draw {
            let walking = actor.activity == Activity::Moving;
            let speed = actor.motion.speed;
            let stride = stride_for(fx.as_ref().and_then(|f| f.0.actors.get(&key)), speed);
            for t in state.tracks.iter_mut() {
                if t.def.continuous {
                    t.continuous(cur, prev, now, state.history.as_ref(), &ground, &mut made);
                } else {
                    t.discrete(cur, prev, now, walking, speed, stride, &mut made);
                }
            }
            // 0x451700 ends with 0x50A710(0).
            if let Some(h) = state.history.as_mut() {
                h.reset = false;
            }
        }
        state.prev = Some(cur);
    }
    tracks.quads.extend(made);
    // 0x7198A0: a quad whose fade end has passed is dropped.
    tracks.quads.retain(|q| !(q.t1 < now));
    // Actors gone from the world lose their tracks (their quads stay).
    if new_tick {
        tracks.actors.retain(|id, _| world.actors.get(*id as usize).is_some_and(|a| a.id == *id));
    }

    // 0x719B40 / 0x7198C0: one mesh per texture.
    let Ok(camera) = cameras.single() else { return };
    let far = camera.translation + camera.forward() * SORT_AHEAD;
    let mut by_texture: HashMap<String, (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[f32; 4]>, Vec<u32>)> = HashMap::new();
    for q in &tracks.quads {
        let e = by_texture.entry(q.texture.clone()).or_default();
        let base = e.0.len() as u32;
        let u = if q.mirrored { 1.0 } else { 0.0 };
        let uv = [[u, q.v[0]], [u, q.v[1]], [1.0 - u, q.v[1]], [1.0 - u, q.v[0]]];
        // Stand-in for the vertex colour 0x71C580 takes from the terrain's
        // light at the first corner: white here (its alpha byte times the
        // fade); the terrain material lights the quad like the ground.
        let a = q.alpha(now);
        for (c, uv) in q.corners.iter().zip(uv) {
            let r = crate::battle_view::to_render(*c) + Vec3::Y * LIFT;
            e.0.push((r - far).to_array());
            e.1.push(uv);
            e.2.push([1.0, 1.0, 1.0, a]);
        }
        e.3.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    for key in tracks.batches.keys().cloned().collect::<Vec<_>>() {
        by_texture.entry(key).or_default();
    }
    for (key, (pos, uvs, colors, indices)) in by_texture {
        if !tracks.batches.contains_key(&key) {
            if tracks.missing.contains(&key) {
                continue;
            }
            let Some(image) = texture_image(&key) else {
                warn!("tracks: texture {key} not found");
                tracks.missing.push(key);
                continue;
            };
            let mut material = crate::look::terrain_material(Some(images.add(image)), [1.0; 4], None, 0, &crate::look::GroundMaps::default());
            material.base.alpha_mode = AlphaMode::Blend;
            material.base.depth_bias = 0.0;
            material.base.cull_mode = None;
            let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD);
            write_mesh(&mut mesh, Vec::new(), Vec::new(), Vec::new(), Vec::new());
            let mesh = meshes.add(mesh);
            let entity = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(materials.add(material)), Transform::from_translation(far), NoFrustumCulling, NotShadowCaster)).id();
            tracks.batches.insert(key.clone(), Batch { entity, mesh });
        }
        let Some(batch) = tracks.batches.get(&key) else { continue };
        if let Some(mut mesh) = meshes.get_mut(&batch.mesh) {
            write_mesh(&mut mesh, pos, uvs, colors, indices);
        }
        if let Ok(mut t) = batch_transforms.get_mut(batch.entity) {
            t.translation = far;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(continuous: bool) -> Arc<TrackDef> {
        Arc::new(TrackDef { continuous, mirrored: false, placement_pcg: 0.5, texture: "t".into(), offset: [0.4, 1.0], size: [0.25, 0.5], fade_sec: 4.0 })
    }

    #[test]
    fn frame_offsets_across_and_along() {
        let t = Track::new(def(true));
        // Driving along native +X: the side is +Y, offset.x moves to -side.
        let (side, center, dir) = t.frame(Vec3::new(1.0, 0.0, 2.0), Vec3::ZERO, true);
        assert_eq!(dir, Vec3::X);
        assert_eq!(side, Vec3::Y);
        assert_eq!(center, Vec3::new(2.0, -0.4, 2.0));
    }

    #[test]
    fn strip_v_runs_by_distance_over_length() {
        let mut t = Track::new(def(true));
        let mut out = Vec::new();
        let ground = |_: Vec2| 0.0f32;
        t.continuous(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 1.0, None, &ground, &mut out);
        // The first call starts the strip at the previous point and lays a
        // quad to the current one: 1 m over a length of 0.5 m.
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].v, [0.0, 2.0]);
        assert_eq!(out[0].t1, 5.0);
        // Not moving: no quad.
        t.continuous(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 1.1, None, &ground, &mut out);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn fade_holds_then_falls() {
        let q = Quad::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, &def(true), 0.0, 1.0, 10.0);
        assert_eq!(q.alpha(10.0), 1.0);
        assert_eq!(q.alpha(12.0), 1.0);
        assert_eq!(q.alpha(13.0), 0.5);
        assert_eq!(q.alpha(14.0), 0.0);
        assert_eq!(q.alpha(20.0), 0.0);
    }

    #[test]
    fn footprints_each_cycle_from_the_placement() {
        let mut t = Track::new(def(false));
        let mut out = Vec::new();
        // stride 1 m, 0.25 m steps: the first call sets the counter to
        // 0.5 - 0.25 and adds 0.25 (0.5), then +0.25 a step: a print when
        // it reaches 1, at steps 3 and 7.
        let mut prev = Vec3::ZERO;
        for i in 1..=8 {
            let cur = Vec3::new(0.25 * i as f32, 0.0, 0.0);
            t.discrete(cur, prev, i as f32, true, 1.0, Some(1.0), &mut out);
            prev = cur;
        }
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].t0, 3.0);
        assert_eq!(out[1].t0, 7.0);
        t.discrete(prev, prev, 9.0, false, 1.0, Some(1.0), &mut out);
        assert_eq!(t.cycle, -1.0);
    }
}

//! The original's view animation of the models battle_view draws:
//! - tank treads: KKC_TankTreadAnimShader. ProcessTankTreads `73CD60`
//!   (model load) takes the largest triangle of a "tred" geometry whose
//!   normal points up and stores which texture axis runs along the model's
//!   forward axis and how many texture units one metre is; `73D1E0` (every
//!   draw) adds the forward distance the geometry's world origin moved
//!   times that ratio to a texture-translation matrix;
//! - tires: KKC_VObjectDynamic. OnResourcesReady `453BF0` finds the Tire
//!   bones, SetPosition `453A50` -> `453600` adds distance /
//!   tire_circumference to each tire's turns (wrapped at 100) and replaces
//!   the bone's local rotation with RotX(2π turns); a FRONT tire is also
//!   turned RotZ(steer), steer = UnitWheelTurnScalar × the heading change of
//!   the last two moves, clamped to ±UnitWheelTurnMaxRadians (AVars);
//! - equipment: FXC_Equipment HIDE / UNHIDE of the geometry under a node
//!   tagged "Eq" (its name is the equipment type);
//! - trailers: KKC_VTrailer::MirrorPosition `45CC40` (the trailer hangs on
//!   the truck's Hitch node, pitched toward it from its rear axle, rolled
//!   with the ground under its rear corners, at most 90° from the truck).
use super::extract;
use aa_formats::nif::NifFile;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Texture offsets a tread mesh is pre-built for (one texture repeat).
pub const TREAD_STEPS: usize = 64;
/// AVars UnitWheelTurnMaxRadians / UnitWheelTurnScalar.
pub const WHEEL_TURN_MAX: f32 = 1.57;
pub const WHEEL_TURN_SCALAR: f32 = 1.3;

/// What one drawn part (converted mesh) is in the original model.
#[derive(Clone, Debug, Default)]
pub struct PartInfo {
    /// Lower-case node names from the root to the geometry.
    pub path: Vec<String>,
    pub tread: Option<Tread>,
    /// The equipment type: the nearest node tagged "Eq", lower case.
    pub equipment: Option<String>,
    /// The geometry's bind origin in model space (NIF frame).
    pub origin: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Tread {
    /// The texture axis along the model's forward axis: V (else U).
    pub v_axis: bool,
    /// Texture units per metre along the model's Y axis.
    pub ratio: f32,
}

/// A node's bind placement: the parent's global times its translation
/// (the frame its rotation turns in), its own rotation, and its global.
#[derive(Clone, Copy, Debug)]
pub struct NodeBind {
    pub pivot: Mat4,
    pub r0: Mat3,
    pub global: Mat4,
}

#[derive(Clone, Debug, Default)]
pub struct ModelInfo {
    pub parts: Vec<PartInfo>,
    /// Lower-case node name -> bind (first of a name).
    pub nodes: HashMap<String, NodeBind>,
}

fn name_of(file: &NifFile, obj: usize) -> String {
    file.objects[obj].fields.string("name").flatten().map(|b| String::from_utf8_lossy(b).to_ascii_lowercase()).unwrap_or_default()
}

/// The extra-data names of an object, lower case.
fn tags(file: &NifFile, obj: usize) -> Vec<String> {
    let f = &file.objects[obj].fields;
    let mut out = Vec::new();
    let mut links: Vec<u32> = f.links("extra_data_list").map(|l| l.to_vec()).unwrap_or_default();
    if let Some(l) = f.link("extra_data") {
        links.push(l);
    }
    let mut seen = 0;
    while let Some(l) = links.pop() {
        seen += 1;
        if seen > 64 {
            break;
        }
        let Some(x) = file.target(l).filter(|&x| x < file.objects.len()) else { continue };
        let xf = &file.objects[x].fields;
        if let Some(Some(n)) = xf.string("extra_name").or_else(|| xf.string("name")) {
            out.push(String::from_utf8_lossy(n).to_ascii_lowercase());
        }
        // NIF 4.x chains extra data through "next_extra_data".
        if let Some(n) = xf.link("next_extra_data") {
            links.push(n);
        }
    }
    out
}

/// The model's parts (`meshes`: the converted meshes battle_view draws, in
/// order) matched to the NIF geometry by node path and vertex count.
pub fn model_info(file: &NifFile, meshes: &[&aa_assets::PreparedMesh]) -> ModelInfo {
    let scene = extract::scene(file);
    let mut obj_of = vec![usize::MAX; scene.nodes.len()];
    for (o, n) in &scene.node_of {
        if *n < obj_of.len() {
            obj_of[*n] = *o;
        }
    }
    let mut globals: Vec<Mat4> = Vec::with_capacity(scene.nodes.len());
    let mut info = ModelInfo::default();
    for (i, n) in scene.nodes.iter().enumerate() {
        let local = n.local(0.0);
        let parent = if n.p >= 0 && (n.p as usize) < i { globals[n.p as usize] } else { Mat4::IDENTITY };
        let g = parent * local;
        globals.push(g);
        let r0 = Mat3::from_quat(Quat::from_xyzw(n.q[0], n.q[1], n.q[2], n.q[3]).normalize());
        let pivot = parent * Mat4::from_translation(Vec3::from(n.t));
        info.nodes.entry(n.name.to_ascii_lowercase()).or_insert(NodeBind { pivot, r0, global: g });
    }
    let chain = |mut n: usize| {
        let mut out = Vec::new();
        loop {
            out.push(n);
            let p = scene.nodes[n].p;
            if p < 0 || p as usize >= n {
                break;
            }
            n = p as usize;
        }
        out.reverse();
        out
    };
    // Every drawn NIF geometry: its node chain and vertex count.
    let target = |f: &aa_formats::nif::Fields, name: &str| f.link(name).and_then(|l| file.target(l)).filter(|&i| i < file.objects.len());
    let geos: Vec<(Vec<usize>, usize)> = scene
        .geometry
        .iter()
        .map(|&(obj, node, _)| {
            let count = target(&file.objects[obj].fields, "data").and_then(|d| file.objects[d].fields.f32s("vertices").map(|v| v.len() / 3)).unwrap_or(0);
            (chain(node), count)
        })
        .collect();
    let names = |c: &[usize]| c.iter().map(|&n| scene.nodes[n].name.to_ascii_lowercase()).collect::<Vec<_>>();
    let mut used = vec![false; geos.len()];
    for m in meshes {
        let path: Vec<String> = m.hierarchy.iter().map(|s| s.to_ascii_lowercase()).collect();
        let count = m.vertices.len();
        let found = geos
            .iter()
            .enumerate()
            .position(|(gi, (c, n))| !used[gi] && *n == count && names(c) == path)
            .or_else(|| geos.iter().enumerate().position(|(gi, (c, n))| !used[gi] && *n == count && names(c).last() == path.last()));
        let mut part = PartInfo { path: path.clone(), ..Default::default() };
        if let Some(gi) = found {
            used[gi] = true;
            let c = &geos[gi].0;
            let node_tags: Vec<Vec<String>> = c.iter().map(|&n| if obj_of[n] != usize::MAX { tags(file, obj_of[n]) } else { Vec::new() }).collect();
            if node_tags.iter().any(|t| t.iter().any(|x| x == "tred")) {
                part.tread = tread(m);
            }
            // The equipment node is the nearest tagged node above the
            // geometry (the geometry repeats the tag).
            let above = c.len().saturating_sub(1);
            part.equipment = c[..above]
                .iter()
                .zip(&node_tags[..above])
                .rev()
                .find(|(_, t)| t.iter().any(|x| x == "eq"))
                .map(|(&n, _)| name_of(file, obj_of[n]))
                .or_else(|| node_tags.last().is_some_and(|t| t.iter().any(|x| x == "eq")).then(|| c.last().map(|&n| name_of(file, obj_of[n]))).flatten());
            if let Some(&last) = c.last() {
                part.origin = globals[last].w_axis.truncate();
            }
        }
        info.parts.push(part);
    }
    info
}

/// ProcessTankTreads `73CD60` on a converted (render-space) mesh.
fn tread(m: &aa_assets::PreparedMesh) -> Option<Tread> {
    let native = |p: [f32; 3]| Vec3::new(p[0], -p[2], p[1]);
    let v = &m.vertices;
    let mut best: Option<([usize; 3], f32)> = None;
    for t in m.indices.chunks_exact(3) {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        if a >= v.len() || b >= v.len() || c >= v.len() {
            continue;
        }
        // the first vertex's normal not pointing down (native z = render y)
        if v[a].normal[1] < 0.0 {
            continue;
        }
        let (pa, pb, pc) = (native(v[a].position), native(v[b].position), native(v[c].position));
        let area = (pb - pa).cross(pc - pa).length();
        if best.is_none_or(|(_, s)| area > s) {
            best = Some(([a, b, c], area));
        }
    }
    let (tri, _) = best?;
    let y = |i: usize| native(v[i].position).y;
    let lo = *tri.iter().min_by(|&&i, &&j| y(i).total_cmp(&y(j)))?;
    let hi = *tri.iter().max_by(|&&i, &&j| y(i).total_cmp(&y(j)))?;
    let span = y(hi) - y(lo);
    if span.abs() < 1e-6 {
        return None;
    }
    let (du, dv) = (v[hi].uv[0] - v[lo].uv[0], v[hi].uv[1] - v[lo].uv[1]);
    Some(if dv.abs() >= du.abs() { Tread { v_axis: true, ratio: dv / span } } else { Tread { v_axis: false, ratio: du / span } })
}

/// The tread mesh at each of TREAD_STEPS texture offsets (one repeat).
pub fn tread_variants(m: &aa_assets::PreparedMesh, t: Tread, meshes: &mut Assets<Mesh>) -> Arc<Vec<Handle<Mesh>>> {
    let mut out = Vec::with_capacity(TREAD_STEPS);
    for k in 0..TREAD_STEPS {
        let shift = k as f32 / TREAD_STEPS as f32;
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, m.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, m.vertices.iter().map(|v| v.normal).collect::<Vec<_>>());
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            m.vertices
                .iter()
                .map(|v| {
                    let uv = if t.v_axis { [v.uv[0], v.uv[1] + shift] } else { [v.uv[0] + shift, v.uv[1]] };
                    crate::scene::model_uv(uv)
                })
                .collect::<Vec<_>>(),
        );
        mesh.insert_indices(Indices::U32(m.indices.clone()));
        out.push(meshes.add(mesh));
    }
    Arc::new(out)
}

/// Native (x, y, z up) -> render (x, z, -y) as a basis change.
pub fn to_render_basis() -> Mat4 {
    Mat4::from_cols(Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::new(0.0, 0.0, -1.0, 0.0), Vec4::new(0.0, 1.0, 0.0, 0.0), Vec4::W)
}

/// The model matrix (native frame) of a model facing `forward` with `up`:
/// columns X = up × forward, Y = -forward, Z = X × Y (models face -Y;
/// `0x70D410(-forward, up)`).
pub fn model_matrix(pos: Vec3, forward: Vec3, up: Vec3) -> Mat4 {
    let y = -forward.normalize_or(Vec3::NEG_Y);
    let x = up.cross(forward).normalize_or(Vec3::X);
    let z = x.cross(y).normalize_or(Vec3::Z);
    Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), pos.extend(1.0))
}

/// A native model matrix as a render-space Transform.
pub fn render_transform(native: Mat4) -> Transform {
    let c = to_render_basis();
    Transform::from_matrix(c * native * c.inverse())
}

struct TireState {
    parts: Vec<usize>,
    a: Mat4,
    a_inv: Mat4,
    r0t: Mat4,
    front: bool,
    turns: f32,
}

struct TreadState {
    part: usize,
    variants: Arc<Vec<Handle<Mesh>>>,
    ratio: f32,
    origin: Vec3,
    offset: f32,
    shown: usize,
}

/// One drawn vehicle's tires and treads.
#[derive(Default)]
pub struct Running {
    tires: Vec<TireState>,
    circumference: f32,
    treads: Vec<TreadState>,
    last: Option<Vec3>,
    last_world: Option<Mat4>,
    /// The last three tick positions (oldest first).
    history: Vec<Vec3>,
}

/// What a frame changes on the parts: tire part transforms (actor-local
/// render space) and tread meshes.
#[derive(Default)]
pub struct Changes {
    pub transforms: Vec<(usize, Transform)>,
    pub meshes: Vec<(usize, Handle<Mesh>)>,
}

impl Running {
    /// `tires`: (node name lower case, front) from the Tire bones.
    pub fn new(info: &ModelInfo, tires: &[(String, bool)], circumference: f32, treads: Vec<(usize, Arc<Vec<Handle<Mesh>>>, Tread)>) -> Option<Self> {
        let mut out = Running { circumference: if circumference > 0.0 { circumference } else { 1.0 }, ..Default::default() };
        for (node, front) in tires {
            let Some(b) = info.nodes.get(node) else { continue };
            let parts: Vec<usize> = info.parts.iter().enumerate().filter(|(_, p)| p.path.iter().any(|n| n == node)).map(|(i, _)| i).collect();
            if parts.is_empty() {
                continue;
            }
            out.tires.push(TireState { parts, a: b.pivot, a_inv: b.pivot.inverse(), r0t: Mat4::from_mat3(b.r0.transpose()), front: *front, turns: 0.0 });
        }
        for (part, variants, t) in treads {
            out.treads.push(TreadState { part, variants, ratio: t.ratio, origin: info.parts[part].origin, offset: 0.0, shown: usize::MAX });
        }
        (!out.tires.is_empty() || !out.treads.is_empty()).then_some(out)
    }

    /// For logs: each tire's turns and steering, each tread's offset.
    pub fn state(&self) -> String {
        format!(
            "tire turns {:?} steer {:.3}, tread offsets {:?}",
            self.tires.iter().map(|t| (t.turns * 100.0).round() / 100.0).collect::<Vec<_>>(),
            self.steer(),
            self.treads.iter().map(|t| (t.offset * 100.0).round() / 100.0).collect::<Vec<_>>()
        )
    }

    /// A new sim tick position (for the front tires' steering).
    pub fn tick(&mut self, pos: Vec3) {
        if self.history.last().is_some_and(|p| p.distance(pos) < 1e-4) {
            return;
        }
        self.history.push(pos);
        if self.history.len() > 3 {
            self.history.remove(0);
        }
    }

    /// The drawn model moved to `world` (native model matrix).
    pub fn update(&mut self, world: Mat4) -> Changes {
        let mut out = Changes::default();
        let pos = world.w_axis.truncate();
        let moved = self.last.map_or(0.0, |l| l.distance(pos));
        // 453600: tires turn by the distance the model moved.
        if moved > 1e-4 || self.last.is_none() {
            let steer = self.steer();
            let c = to_render_basis();
            let ci = c.inverse();
            for t in &mut self.tires {
                t.turns += moved / self.circumference;
                if t.turns > 100.0 {
                    t.turns -= 100.0;
                }
                let spin = Mat4::from_rotation_x(std::f32::consts::TAU * t.turns);
                let r = if t.front { Mat4::from_rotation_z(steer) * spin } else { spin };
                let m = c * (t.a * r * t.r0t * t.a_inv) * ci;
                let tf = Transform::from_matrix(m);
                for &p in &t.parts {
                    out.transforms.push((p, tf));
                }
            }
        }
        // 73D1E0: the tread's world origin, forward along the model.
        if let Some(prev) = self.last_world {
            let forward = (world.transform_vector3(Vec3::NEG_Y)).normalize_or_zero();
            for t in &mut self.treads {
                let now = world.transform_point3(t.origin);
                let before = prev.transform_point3(t.origin);
                t.offset = (t.offset + forward.dot(now - before) * t.ratio).rem_euclid(1.0);
                let k = ((t.offset * TREAD_STEPS as f32) as usize).min(TREAD_STEPS - 1);
                if k != t.shown {
                    t.shown = k;
                    out.meshes.push((t.part, t.variants[k].clone()));
                }
            }
        }
        self.last = Some(pos);
        self.last_world = Some(world);
        out
    }

    /// The front tires' steering: the heading change from the move before
    /// the last to the last one (453600 / 452B20), scaled and clamped.
    fn steer(&self) -> f32 {
        let [p0, p1, p2] = match self.history.as_slice() {
            [a, b, c] => [*a, *b, *c],
            _ => return 0.0,
        };
        let v1 = (p1 - p0).truncate();
        let v2 = (p2 - p1).truncate();
        if v1.length_squared() < 1e-12 || v2.length_squared() < 1e-12 {
            return 0.0;
        }
        let angle = v1.angle_to(v2);
        (angle * WHEEL_TURN_SCALAR).clamp(-WHEEL_TURN_MAX, WHEEL_TURN_MAX)
    }
}

/// rot2D of the original (data x right, y forward) by a facing vector.
pub fn rot2d(a: Vec2, f: Vec2) -> Vec2 {
    Vec2::new(a.x * f.y + a.y * f.x, a.y * f.y - a.x * f.x)
}

/// The 90° limit of `45CA00`: a facing pointing backward from the truck's
/// is turned to the nearer side.
fn clamp_facing(fc: Vec3, f: Vec2) -> Vec3 {
    if fc.truncate().dot(f) >= 0.0 {
        return fc;
    }
    let p = Vec2::new(f.y, -f.x).normalize_or_zero();
    let side = if fc.truncate().dot(p) >= 0.0 { p } else { -p };
    Vec3::new(side.x, side.y, fc.z).normalize_or(Vec3::new(side.x, side.y, 0.0))
}

/// A towed trailer's view placement (KKC_VTrailer::MirrorPosition).
pub struct Trailer {
    pub pos: Vec3,
    pub facing: Vec3,
    pub up: Vec3,
    /// The trailer model's Hitch node translation (model space, NIF frame).
    pub hitch: Vec3,
    /// Soft-shape rear corners in data space (x right, y forward).
    pub rear: [Vec2; 2],
}

impl Trailer {
    /// Spawn placement (`4D6F10`): behind the truck by the two TrailerHitch
    /// offsets, with the truck's height and facing.
    pub fn spawn(truck_pos: Vec3, truck_facing: Vec2, truck_offset: [f32; 3], trailer_offset: [f32; 3], hitch: Vec3, rear: [Vec2; 2]) -> Self {
        let d = rot2d(Vec2::new(truck_offset[0] - trailer_offset[0], truck_offset[1] - trailer_offset[1]), truck_facing);
        Trailer { pos: Vec3::new(truck_pos.x + d.x, truck_pos.y + d.y, truck_pos.z), facing: truck_facing.extend(0.0), up: Vec3::Z, hitch, rear }
    }

    /// One frame: `h` the truck's Hitch point (world), `f` the truck's
    /// facing (xy), `ground` the terrain height, `paused` no time passed.
    pub fn mirror(&mut self, h: Vec3, f: Vec2, ground: impl Fn(Vec2) -> f32, paused: bool) -> bool {
        let model = model_matrix(self.pos, self.facing, self.up);
        let t = model.transform_point3(self.hitch);
        if (t - h).abs().element_sum() < 0.01 {
            return false;
        }
        let fc = clamp_facing(self.facing, f);
        let u = self.up;
        let hz = self.hitch.z;
        let mut p = h - u * hz;
        p.z = ground(p.truncate());
        let d = (h - p).dot(u) - hz;
        let q = Vec3::new(p.x, p.y, p.z + d);
        let corner = |c: Vec2| {
            let w = self.pos.truncate() + rot2d(c, fc.truncate().normalize_or(Vec2::Y));
            Vec3::new(w.x, w.y, ground(w))
        };
        let c1 = corner(self.rear[0]);
        let c2 = corner(self.rear[1]);
        let m = (c1 + c2) * 0.5;
        if paused {
            return false;
        }
        let fn_ = clamp_facing((q - m).normalize_or(fc), f);
        let up = (c2 - c1).cross(fn_).normalize_or(Vec3::Z);
        let rt = fn_.cross(up);
        self.pos = h + self.hitch.x * rt + self.hitch.y * fn_ - self.hitch.z * up;
        self.facing = fn_;
        self.up = up;
        true
    }

    pub fn matrix(&self) -> Mat4 {
        model_matrix(self.pos, self.facing, self.up)
    }
}

/// An original model with its own (embedded) animation baked at 20 Hz:
/// per part the frame meshes and the material.
pub struct AnimModel {
    pub parts: Vec<(Vec<Handle<Mesh>>, Handle<crate::look::OriginalMaterial>)>,
    pub frames: usize,
    pub looping: bool,
    /// The timeline's length (s); playback steps frames at 20 Hz.
    #[allow(dead_code)]
    pub duration: f32,
    /// Per part its texture path and material (aspect skins).
    pub src: Vec<(Option<String>, aa_assets::Material)>,
}

type Decoded = (Vec<(Vec<Mesh>, aa_assets::Material, Option<Image>)>, usize, bool, f32);

enum Slot {
    Loading(Arc<std::sync::Mutex<Option<Option<Decoded>>>>),
    Ready(Option<Arc<AnimModel>>),
}

/// Aspect models (building pack / unpack / death, animated normal models)
/// decoded and baked on loader threads, so a deployment never stalls a frame.
pub struct AnimatedModels {
    store: Arc<aa_assets::AssetStore>,
    fixes: Arc<crate::model_fixes::ModelFixes>,
    data_root: std::path::PathBuf,
    slots: HashMap<(String, [u8; 3]), Slot>,
    pub failures: Vec<String>,
}

impl AnimatedModels {
    pub fn new(store: Arc<aa_assets::AssetStore>, fixes: Arc<crate::model_fixes::ModelFixes>) -> Self {
        let data_root = super::reconstruction_root().map(|r| r.join("assets/original/Data")).unwrap_or_default();
        Self { store, fixes, data_root, slots: HashMap::new(), failures: Vec::new() }
    }

    /// Start decoding `path` (Data-relative) for `team`.
    pub fn request(&mut self, path: &str, team: [u8; 3]) {
        let key = (path.to_ascii_lowercase(), team);
        if self.slots.contains_key(&key) {
            return;
        }
        let slot = Arc::new(std::sync::Mutex::new(None));
        let out = slot.clone();
        let (store, fixes, root, path) = (self.store.clone(), self.fixes.clone(), self.data_root.clone(), path.to_owned());
        rayon::spawn(move || {
            let decoded = (|| -> Option<Decoded> {
                let mut model = store.load_model(&path).ok()?.prepare().ok()?;
                let _ = fixes.apply(&store, &mut model);
                let meshes: Vec<&aa_assets::PreparedMesh> = model.meshes.iter().filter(|m| !m.indices.is_empty()).collect();
                let converted: Vec<(String, usize)> = meshes.iter().map(|m| (m.name.clone(), m.vertices.len())).collect();
                let baked = super::bones::find_file(&root, &path).and_then(|p| std::fs::read(p).ok()).and_then(|b| aa_formats::nif::load(&b).ok()).and_then(|f| super::anim::bake(&f, &converted, false));
                let mut parts = Vec::new();
                for (i, mesh) in meshes.iter().enumerate() {
                    let texture = mesh.material.texture.as_ref().and_then(|t| crate::textures::load(&store, t, crate::scene::kind(&mesh.material, team)).ok());
                    let frames = match baked.as_ref().and_then(|b| b.parts.get(i)).and_then(|p| p.frames.as_ref()) {
                        Some(frames) => frames
                            .iter()
                            .map(|(pos, nrm)| {
                                let mut m = crate::scene::model_mesh(mesh);
                                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos.clone());
                                m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.clone());
                                m
                            })
                            .collect(),
                        None => vec![crate::scene::model_mesh(mesh)],
                    };
                    parts.push((frames, mesh.material.clone(), texture));
                }
                Some((parts, baked.as_ref().map_or(1, |b| b.frames), baked.as_ref().is_some_and(|b| b.looping), baked.as_ref().map_or(0.0, |b| b.duration)))
            })();
            *out.lock().unwrap() = Some(decoded);
        });
        self.slots.insert(key, Slot::Loading(slot));
    }

    /// The model: None while decoding, Some(None) if it failed.
    pub fn get(&mut self, path: &str, team: [u8; 3], meshes: &mut Assets<Mesh>, materials: &mut Assets<crate::look::OriginalMaterial>, images: &mut Assets<Image>) -> Option<Option<Arc<AnimModel>>> {
        self.request(path, team);
        let key = (path.to_ascii_lowercase(), team);
        let decoded = match self.slots.get(&key)? {
            Slot::Ready(m) => return Some(m.clone()),
            Slot::Loading(slot) => slot.lock().unwrap().take()?,
        };
        let model = decoded.map(|(list, frames, looping, duration)| {
            let src = list.iter().map(|(_, m, _)| (m.texture.clone(), m.clone())).collect();
            Arc::new(AnimModel {
                src,
                parts: list
                    .into_iter()
                    .map(|(frames, material, image)| {
                        let texture = image.map(|i| images.add(i));
                        (frames.into_iter().map(|m| meshes.add(m)).collect::<Vec<_>>(), materials.add(crate::scene::standard(&material, texture)))
                    })
                    .collect(),
                frames,
                looping,
                duration,
            })
        });
        if model.is_none() {
            self.failures.push(format!("aspect model {path} did not load"));
        }
        self.slots.insert(key, Slot::Ready(model.clone()));
        Some(model)
    }
}

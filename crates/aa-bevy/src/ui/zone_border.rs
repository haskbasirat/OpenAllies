//! The friendly supply zone border, as the original's zone renderer draws it
//! (KKC_RZones `0x72EC90`: zone type 0, "friendly supply").
//!
//! * The zone is the local kingdom's supply zone tiles. The original asks
//!   for zone membership per terrain vertex (`0x72C0D0` -> `0x72B6B0`); we
//!   use the zone tiles of the supply flood, so the border runs along tile
//!   edges.
//! * Border segments (`0x72E2F0`): one axis-aligned segment wherever two
//!   neighbouring tiles of the map differ in membership. Segments are at the
//!   ground height plus AVar TerrainZoneHeightBias (0.2, `0x401BD0`).
//! * The minimap draws these raw segments (`0x433470`: [`segments`]).
//! * The 3D world chains them into polylines (`0x72CC10` / `0x72C9F0`) and
//!   smooths them with the uniform cubic B-spline (basis `0x72B830`,
//!   evaluation `0x55DA10`) at t = 0 and 0.5 of every span (`0x72CE20`);
//!   each sample is set back on the ground plus the bias (the original
//!   splines the heights too, but its line runs unbroken over hills, ours
//!   would dip under the depth-tested terrain).
//!   Each polyline then becomes a ribbon strip (`0x72C290` / `0x72B6D0`):
//!   - AVar ViewZoneFriendlySupplyWidth wide (AVars_AA 1.25), across the
//!     direction from the previous to the next point;
//!   - texture ViewZoneFriendlySupplyTexture (textures/special/
//!     ZoneBorder.tga: a solid and a dashed stripe), u 0..1 across, v
//!     along the length, so the texture repeats a whole number of times,
//!     each about one width long;
//!   - colour ViewZoneFriendlySupplyColor (AVars_AA 0,1,0).
//!   The inside edge takes u = 0: the solid stripe runs on the zone side,
//!   as in the original's screenshots.
//! * Both are shown while UVar ViewShowZoneFriendlySupply is set (`0x72B8A0`;
//!   default true, toggled by ButtonSupplyBorder, ToggleShowZoneSupply).
//! * The ribbon is refreshed when the zone tiles change, checked every AVar
//!   MinimapPeriodUpdateSupply seconds.
use crate::session::Battle;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;

/// AVar TerrainZoneHeightBias (hard-coded default, no data file sets it).
pub const HEIGHT_BIAS: f32 = 0.2;

/// The local kingdom's supply tiles (map cell index -> inside).
pub fn supply_tiles(b: &Battle) -> Vec<bool> {
    let m = &b.world.map;
    let mut inside = vec![false; (m.width * m.height) as usize];
    for (i, z) in b.world.books.zones.iter().enumerate() {
        if z.kingdom != b.local {
            continue;
        }
        for &tile in b.world.books.zone_tiles.get(i).map_or(&[][..], |s| s.as_slice()) {
            if let Some(v) = inside.get_mut(tile as usize) {
                *v = true;
            }
        }
    }
    inside
}

/// The border segments between neighbouring map tiles of different
/// membership, in tile-corner coordinates, oriented with the inside tile on
/// the left.
pub fn segments(width: u32, height: u32, inside: &[bool]) -> Vec<(IVec2, IVec2)> {
    let (w, h) = (width as i32, height as i32);
    let at = |x: i32, y: i32| inside.get((y * w + x) as usize).copied().unwrap_or(false);
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            // the edge to the right neighbour (vertical) and to the upper
            // neighbour (horizontal); both tiles on the map
            if x + 1 < w && at(x, y) != at(x + 1, y) {
                let (a, b) = (IVec2::new(x + 1, y), IVec2::new(x + 1, y + 1));
                out.push(if at(x, y) { (a, b) } else { (b, a) });
            }
            if y + 1 < h && at(x, y) != at(x, y + 1) {
                let (a, b) = (IVec2::new(x + 1, y + 1), IVec2::new(x, y + 1));
                out.push(if at(x, y) { (a, b) } else { (b, a) });
            }
        }
    }
    out
}

/// The segments chained into polylines (`0x72CC10` / `0x72C9F0`): closed
/// loops where they close, open chains where they end at the map edge.
fn chains(segs: &[(IVec2, IVec2)]) -> Vec<(Vec<IVec2>, bool)> {
    let mut from: HashMap<IVec2, Vec<usize>> = HashMap::new();
    let mut to: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        from.entry(*a).or_default().push(i);
        to.entry(*b).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    for s in 0..segs.len() {
        if used[s] {
            continue;
        }
        // back to the start of an open chain (a loop comes back to s)
        let mut first = s;
        for _ in 0..segs.len() {
            let prev = to.get(&segs[first].0).and_then(|l| l.iter().copied().find(|&j| !used[j] && j != first));
            match prev {
                Some(p) if p != s => first = p,
                _ => break,
            }
        }
        let start = segs[first].0;
        let mut pts = vec![start];
        let mut cur = first;
        let mut closed = false;
        loop {
            used[cur] = true;
            let (a, b) = segs[cur];
            if b == start {
                closed = true;
                break;
            }
            pts.push(b);
            // the next segment from b; at a corner with two, the left turn
            let dir = b - a;
            let next = from.get(&b).and_then(|list| {
                list.iter().copied().filter(|&j| !used[j]).max_by_key(|&j| {
                    let d = segs[j].1 - segs[j].0;
                    (dir.x * d.y - dir.y * d.x, dir.x * d.x + dir.y * d.y)
                })
            });
            match next {
                Some(n) => cur = n,
                None => break,
            }
        }
        if pts.len() >= 2 {
            out.push((pts, closed));
        }
    }
    out
}

/// Uniform cubic B-spline (`0x72B830` basis, `0x55DA10`).
fn bspline(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let b0 = ((-1.0 / 6.0 * t + 0.5) * t - 0.5) * t + 1.0 / 6.0;
    let b1 = ((0.5 * t - 1.0) * t) * t + 2.0 / 3.0;
    let b2 = ((-0.5 * t + 0.5) * t + 0.5) * t + 1.0 / 6.0;
    let b3 = t * t * t / 6.0;
    p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3
}

/// `0x72CE20`: two samples (t = 0, 0.5) per control point; the last span
/// of an open chain also its end (t = 1).
fn smooth(points: &[Vec3], closed: bool) -> Vec<Vec3> {
    let n = points.len() as i32;
    let get = |i: i32| -> Vec3 {
        if closed {
            points[i.rem_euclid(n) as usize]
        } else {
            points[i.clamp(0, n - 1) as usize]
        }
    };
    let mut out = Vec::new();
    let spans = if closed { n } else { n + 1 };
    for k in 1..=spans {
        let i = k - 1;
        let (p0, p1, p2, p3) = (get(i - 2), get(i - 1), get(i), get(i + 1));
        let last = k == spans && !closed;
        let mut t = 0.0;
        while t < if last { 1.5 } else { 1.0 } {
            out.push(bspline(p0, p1, p2, p3, t));
            t += 0.5;
        }
    }
    out
}

/// Native (x, y, z up) -> render (x, z, -y).
fn to_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

/// The zone border ribbons in 3D.
#[derive(Resource, Default)]
pub struct ZoneBorder {
    entity: Option<Entity>,
    material: Option<Handle<StandardMaterial>>,
    tiles: Vec<bool>,
    checked_at: Option<f32>,
    /// Map id the ribbon was built for.
    map: Option<String>,
    looks: Option<Looks>,
}

#[derive(Clone)]
struct Looks {
    width: f32,
    color: [f32; 3],
    period: f32,
    texture: Option<Handle<Image>>,
}

fn load_looks(data: &aa_ui::fs::DataRoot, images: &mut Assets<Image>) -> Looks {
    // AVars_AA overrides AVars (the AA layer of the rules)
    let files = [super::datafile::load(data, "AVars_AA.tgi"), super::datafile::load(data, "AVars.tgi")];
    let get = |name: &str| files.iter().flat_map(|f| f.iter()).find_map(|n| n.get(name).map(str::to_string));
    let width = get("ViewZoneFriendlySupplyWidth").and_then(|v| v.trim().parse().ok()).unwrap_or(0.5);
    let color = get("ViewZoneFriendlySupplyColor")
        .and_then(|v| {
            let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (p.len() >= 3).then(|| [p[0], p[1], p[2]])
        })
        .unwrap_or([0.0, 1.0, 0.0]);
    let period = get("MinimapPeriodUpdateSupply").and_then(|v| v.split(';').next().and_then(|s| s.trim().parse().ok())).unwrap_or(5.0);
    let path = get("ViewZoneFriendlySupplyTexture").unwrap_or_else(|| "textures/special/ZoneBorder.tga".into());
    let texture = data.find(path.trim().trim_start_matches('/')).and_then(|p| image::open(p).ok()).map(|img| {
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            img.into_raw(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            mag_filter: bevy::image::ImageFilterMode::Linear,
            min_filter: bevy::image::ImageFilterMode::Linear,
            mipmap_filter: bevy::image::ImageFilterMode::Linear,
            ..default()
        });
        images.add(image)
    });
    if texture.is_none() {
        warn!("zone border: texture {path} not found");
    }
    Looks { width, color, period, texture }
}

/// One mesh with every ribbon (`0x72C290`).
/// `ground(x, z)`: the drawn terrain's height at a render point.
fn ribbon_mesh(b: &Battle, inside: &[bool], width: f32, ground: &dyn Fn(f32, f32) -> f32) -> Option<Mesh> {
    let m = &b.world.map;
    let cs = m.cell_size;
    let segs = segments(m.width, m.height, inside);
    if segs.is_empty() {
        return None;
    }
    let (mut pos, mut uv, mut idx) = (Vec::<[f32; 3]>::new(), Vec::<[f32; 2]>::new(), Vec::<u32>::new());
    for (chain, closed) in chains(&segs) {
        let control: Vec<Vec3> = chain
            .iter()
            .map(|c| {
                let p = Vec2::new(c.x as f32 * cs, c.y as f32 * cs);
                Vec3::new(p.x, p.y, m.height_at(p) + HEIGHT_BIAS)
            })
            .collect();
        // the smoothed points stand on the ground (+ the bias): the spline
        // of the corner heights would cut into convex slopes, which our
        // depth-tested terrain then hides (the original's line runs
        // unbroken over its hills)
        let smoothed: Vec<Vec3> = smooth(&control, closed).into_iter().map(|p| Vec3::new(p.x, p.y, m.height_at(p.truncate()) + HEIGHT_BIAS)).collect();
        // a loop's strip closes on itself: every sample is an interior
        // point, the first one once more at the end
        let pts: Vec<Vec3> = if closed && smoothed.len() >= 2 {
            let m = smoothed.len();
            std::iter::once(smoothed[m - 1]).chain(smoothed.iter().copied()).chain([smoothed[0], smoothed[1]]).collect()
        } else {
            smoothed
        };
        if pts.len() < 3 {
            continue;
        }
        // the strip over the interior points; v repeats the texture a whole
        // number of times, each about one width long
        let n = pts.len();
        let lens: Vec<f32> = (1..n).map(|i| pts[i].distance(pts[i - 1])).collect();
        let total: f32 = lens[1..n - 2].iter().sum::<f32>().max(1e-3);
        let tiles = (total / width + 0.5).floor().max(1.0);
        let scale = tiles / total;
        let base = pos.len() as u32;
        let mut v = 0.0f32;
        for i in 1..n - 1 {
            let d = pts[i + 1] - pts[i - 1];
            let d = d.normalize_or_zero();
            // 0x72B6D0: across the direction, on the ground plane
            let right = Vec3::new(d.y, -d.x, 0.0).normalize_or_zero() * (width * 0.5);
            // each edge on the drawn ground plus the bias (the strip lies on
            // the slope instead of cutting into its uphill side)
            let on_ground = |p: Vec3| {
                let r = to_render(p);
                Vec3::new(r.x, ground(r.x, r.z) + HEIGHT_BIAS, r.z)
            };
            pos.push(on_ground(pts[i] - right).to_array());
            pos.push(on_ground(pts[i] + right).to_array());
            uv.push([0.0, v]);
            uv.push([1.0, v]);
            if i + 1 < n - 1 {
                v += lens[i] * scale;
            }
        }
        let count = (n - 2) as u32;
        for k in 0..count.saturating_sub(1) {
            let a = base + k * 2;
            idx.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    if idx.is_empty() {
        return None;
    }
    let normals = vec![[0.0, 1.0, 0.0]; pos.len()];
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    Some(mesh)
}

/// Keep the ribbon of the local kingdom's supply zones up to date.
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    front: Res<super::front::Front>,
    mut state: ResMut<ZoneBorder>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut visibility: Query<&mut Visibility>,
    ground: Option<Res<crate::camera::Ground>>,
) {
    let Some(b) = battle else {
        if let Some(e) = state.entity.take() {
            commands.entity(e).despawn();
        }
        state.tiles.clear();
        state.checked_at = None;
        state.map = None;
        return;
    };
    let looks = match &state.looks {
        Some(l) => l.clone(),
        None => {
            let l = load_looks(&front.ui.store.root, &mut images);
            state.looks = Some(l.clone());
            l
        }
    };
    let show = front.ui.uvars.get_bool("ViewShowZoneFriendlySupply");
    if let Some(e) = state.entity {
        if let Ok(mut v) = visibility.get_mut(e) {
            let want = if show { Visibility::Inherited } else { Visibility::Hidden };
            if *v != want {
                *v = want;
            }
        }
    }
    let now = b.world.time;
    let new_map = state.map.as_deref() != Some(b.world.map.id.as_str());
    if !new_map && state.checked_at.is_some_and(|t| now - t < looks.period && now >= t) {
        return;
    }
    state.checked_at = Some(now);
    let tiles = supply_tiles(&b);
    if !new_map && tiles == state.tiles {
        return;
    }
    state.map = Some(b.world.map.id.clone());
    let map = &b.world.map;
    let height = |x: f32, z: f32| match &ground {
        Some(g) => (g.0)(x, z),
        None => map.height_at(Vec2::new(x, -z)),
    };
    let mesh = ribbon_mesh(&b, &tiles, looks.width, &height);
    state.tiles = tiles;
    if let Some(e) = state.entity.take() {
        commands.entity(e).despawn();
    }
    let Some(mesh) = mesh else { return };
    let material = state
        .material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(looks.color[0], looks.color[1], looks.color[2]),
                base_color_texture: looks.texture.clone(),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                double_sided: true,
                fog_enabled: false,
                ..default()
            })
        })
        .clone();
    let e = commands
        .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            if show { Visibility::Inherited } else { Visibility::Hidden },
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ))
        .id();
    state.entity = Some(e);
}

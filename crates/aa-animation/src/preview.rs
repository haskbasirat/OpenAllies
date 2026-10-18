//! Rich authored curves for the asset inspector only. Legacy and runtime
//! entry points retain their prior strict linear/constant-bind domain.
use super::*;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClipExtras {
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub alpha_tracks: Vec<AlphaTrack>,
    #[serde(default)]
    pub unsupported_controllers: Vec<serde_json::Value>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlphaTrack {
    pub meshes: Vec<usize>,
    pub target: String,
    pub controller: String,
    pub frequency: f32,
    pub phase: f32,
    pub start: f32,
    pub stop: f32,
    pub flags: u32,
    pub channel: Channel<f32>,
}

pub(crate) fn channel_valid<T>(c: &Channel<T>, check: impl Fn(&T) -> bool) -> bool {
    if c.keys.is_empty() {
        return true;
    }
    if !(1..=3).contains(&c.interpolation) || !c.keys.windows(2).all(|p| p[0].time < p[1].time) {
        return false;
    }
    c.keys.iter().all(|k| {
        k.time.is_finite()
            && check(&k.value)
            && match c.interpolation {
                1 => true,
                2 => {
                    k.forward.as_ref().is_some_and(&check)
                        && k.backward.as_ref().is_some_and(&check)
                }
                3 => [k.tension, k.bias, k.continuity]
                    .iter()
                    .all(|x| x.is_some_and(f32::is_finite)),
                _ => false,
            }
    })
}
fn segment<T>(c: &Channel<T>, time: f32) -> Option<(usize, usize, f32)> {
    let first = c.keys.first()?;
    let next = c.keys.partition_point(|k| k.time <= time);
    if next == 0 {
        return Some((0, 0, first.time));
    }
    if next == c.keys.len() {
        return Some((next - 1, next - 1, 0.0));
    }
    let a = next - 1;
    Some((
        a,
        next,
        (time - c.keys[a].time) / (c.keys[next].time - c.keys[a].time),
    ))
}
fn hermite(a: f32, b: f32, out: f32, input: f32, u: f32) -> f32 {
    // Native type2 8B7CE0 consumes authored tangent values, with no dt factor.
    let u2 = u * u;
    let u3 = u2 * u;
    (2.0 * u3 - 3.0 * u2 + 1.0) * a
        + (-2.0 * u3 + 3.0 * u2) * b
        + (u3 - 2.0 * u2 + u) * out
        + (u3 - u2) * input
}
fn tcb_tangent<T>(c: &Channel<T>, i: usize, get: impl Fn(&T) -> f32, out: bool) -> f32 {
    let k = &c.keys[i];
    let value = get(&k.value);
    let prev = if i == 0 {
        2.0 * value - get(&c.keys[(i + 1).min(c.keys.len() - 1)].value)
    } else {
        get(&c.keys[i - 1].value)
    };
    let next = if i + 1 == c.keys.len() {
        2.0 * value - prev
    } else {
        get(&c.keys[i + 1].value)
    };
    let t = k.tension.expect("validated TCB tension");
    let b = k.bias.expect("validated TCB bias");
    let z = k.continuity.expect("validated TCB continuity");
    let (p, n) = if out {
        ((1.0 + z) * (1.0 + b), (1.0 - z) * (1.0 - b))
    } else {
        ((1.0 + z) * (1.0 - b), (1.0 - z) * (1.0 + b))
    };
    let interval = if i > 0 && i + 1 < c.keys.len() {
        let previous = k.time - c.keys[i - 1].time;
        let next = c.keys[i + 1].time - k.time;
        2.0 * (if out { next } else { previous }) / (previous + next)
    } else {
        1.0
    };
    interval * (1.0 - t) * 0.5 * (p * (value - prev) + n * (next - value))
}
fn component<T>(c: &Channel<T>, time: f32, get: impl Fn(&T) -> f32 + Copy) -> Option<f32> {
    let (a, b, u) = segment(c, time)?;
    let x = get(&c.keys[a].value);
    if a == b {
        return Some(x);
    }
    let y = get(&c.keys[b].value);
    Some(match c.interpolation {
        1 => x + (y - x) * u,
        // Ni*Key reader stores serialized forward at+C, backward at+10.
        // Runtime evaluates left+10 (backward) and right+C (forward).
        2 => hermite(
            x,
            y,
            get(c.keys[a].backward.as_ref().expect("validated tangent")),
            get(c.keys[b].forward.as_ref().expect("validated tangent")),
            u,
        ),
        3 => hermite(
            x,
            y,
            tcb_tangent(c, a, get, true),
            tcb_tangent(c, b, get, false),
            u,
        ),
        _ => unreachable!("validated curve"),
    })
}
pub(crate) fn scalar(c: &Channel<f32>, t: f32) -> Option<f32> {
    component(c, t, |x| *x)
}
pub(crate) fn vector(c: &Channel<[f32; 3]>, t: f32) -> Option<Vec3> {
    if c.keys.is_empty() {
        None
    } else {
        Some(Vec3::from_array(std::array::from_fn(|i| {
            component(c, t, |v| v[i]).unwrap()
        })))
    }
}
pub(crate) fn quaternion(c: &Channel<[f32; 4]>, t: f32) -> Result<Option<[f32; 4]>> {
    if c.interpolation == 1 || c.keys.len() <= 1 {
        return Ok(super::sample_quaternion(c, t));
    }
    // Nonlinear quaternion curves are not component-wise Euler or linear keys.
    Err(invalid(
        "nonlinear quaternion curve requires its authored quaternion interpolation",
    ))
}
pub(crate) fn local(node: &RigNode, track: Option<&NodeTrack>, time: f32) -> Result<Mat4> {
    let mut translation = Vec3::from(node.translation);
    let mut scale = node.scale;
    let mut rotation = Mat3::from_cols_array_2d(&node.rotation);
    if let Some(track) = track {
        let t = time.clamp(track.start, track.stop);
        if let Some(v) = vector(&track.translation, t) {
            translation = v;
        }
        if let Some(v) = scalar(&track.scale, t) {
            scale = if v > 0.0 { v } else { 0.0 };
        }
        match &track.rotation {
            RotationChannel::Quaternion { channel } => {
                if let Some(q) = quaternion(channel, t)? {
                    rotation = super::quaternion_matrix(q)
                }
            }
            RotationChannel::ConstantBind { .. } => {}
            RotationChannel::EulerXyz { channels, order } => {
                let angles = [
                    scalar(&channels[0], t),
                    scalar(&channels[1], t),
                    scalar(&channels[2], t),
                ];
                let a = angles.map(|x| x.unwrap_or(0.0)); // Native862FE0 empty axis literal angle0.
                let axes = [
                    Mat3::from_rotation_x(a[0]),
                    Mat3::from_rotation_y(a[1]),
                    Mat3::from_rotation_z(a[2]),
                ];
                let orders = [
                    [0, 1, 2],
                    [0, 2, 1],
                    [1, 2, 0],
                    [1, 0, 2],
                    [2, 0, 1],
                    [2, 1, 0],
                    [0, 1, 0],
                    [1, 2, 1],
                    [2, 0, 2],
                ];
                rotation = Mat3::IDENTITY;
                for i in orders[*order as usize] {
                    rotation = axes[i] * rotation;
                }
            }
        }
    }
    Ok(Mat4::from_cols(
        (rotation.x_axis * scale).extend(0.0),
        (rotation.y_axis * scale).extend(0.0),
        (rotation.z_axis * scale).extend(0.0),
        translation.extend(1.0),
    ))
}
pub(crate) fn validate_extras(asset: &AnimatedAsset, clip: &Clip) -> Result<()> {
    for t in &clip.preview.alpha_tracks {
        if t.controller != "NiAlphaController"
            || t.meshes.is_empty()
            || t.meshes.iter().any(|&m| m >= asset.meshes.len())
            || t.meshes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != t.meshes.len()
            || t.frequency != 1.0
            || t.phase != 0.0
            || !finite([t.start, t.stop])
            || t.stop < t.start
            || !channel_valid(&t.channel, |x| x.is_finite())
        {
            return Err(invalid("unsupported inspector alpha track"));
        }
    }
    Ok(())
}
pub(crate) fn opacity(clip: Option<&Clip>, mesh: usize, time: f32) -> Option<f32> {
    let mut value = None;
    if let Some(clip) = clip {
        for t in &clip.preview.alpha_tracks {
            if t.meshes.contains(&mesh) {
                if let Some(x) = scalar(&t.channel, time.clamp(t.start, t.stop)) {
                    value = Some(x);
                }
            }
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn curve(rows: &serde_json::Value, vector: bool) -> serde_json::Value {
        let keys:Vec<_>=rows.as_array().unwrap().iter().map(|r| {
            let v=r.as_array().unwrap();
            if vector { serde_json::json!({"time":v[0],"value":[v[1],v[2],v[3]],"forward":[v[4],v[5],v[6]],"backward":[v[7],v[8],v[9]]}) }
            else {serde_json::json!({"time":v[0],"value":v[1],"forward":v[2],"backward":v[3]})}
        }).collect();
        serde_json::json!({"interpolation":2,"keys":keys})
    }
    #[test]
    fn rich_hermite_and_all_euler_orders_match_original_native_probes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/animation-browser-20261017/native-curves/native-curves-lab.json"
        ))
        .unwrap();
        let mut count = 0;
        for c in fixture["cases"].as_array().unwrap() {
            let time = c["time"].as_f64().unwrap() as f32;
            match c["kind"].as_str().unwrap() {
                "scalar2" => {
                    let ch: Channel<f32> =
                        serde_json::from_value(curve(&c["rows"], false)).unwrap();
                    let a = scalar(&ch, time).unwrap();
                    let e = f32::from_bits(c["output"][0].as_u64().unwrap() as u32);
                    assert!((a - e).abs() < 2e-5, "scalar {a} != {e}");
                }
                "vector2" => {
                    let ch: Channel<[f32; 3]> =
                        serde_json::from_value(curve(&c["rows"], true)).unwrap();
                    let a = vector(&ch, time).unwrap().to_array();
                    for (i, v) in a.iter().enumerate() {
                        let e = f32::from_bits(c["output"][0][i].as_u64().unwrap() as u32);
                        assert!((*v - e).abs() < 2e-5);
                    }
                }
                "euler2" => {
                    let channels: Vec<_> = c["axes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|a| curve(a, false))
                        .collect();
                    let t:NodeTrack=serde_json::from_value(serde_json::json!({"node":0,"target":"node","controller":"NiKeyframeController","frequency":1,"phase":0,"start":0,"stop":2,"flags":44,"translation":{"interpolation":1,"keys":[]},"scale":{"interpolation":1,"keys":[]},"rotation":{"kind":"euler_xyz","channels":channels,"order":c["order"]}})).unwrap();
                    let n = RigNode {
                        name: "node".into(),
                        parent: None,
                        translation: [0.0; 3],
                        rotation: Mat3::IDENTITY.to_cols_array_2d(),
                        scale: 1.0,
                    };
                    let a = Mat3::from_mat4(local(&n, Some(&t), time).unwrap());
                    let q = &c["output"][0];
                    let wxyz: [f32; 4] =
                        std::array::from_fn(|i| f32::from_bits(q[i].as_u64().unwrap() as u32));
                    let e = super::super::quaternion_matrix([wxyz[1], wxyz[2], wxyz[3], wxyz[0]]);
                    let error = a
                        .to_cols_array()
                        .iter()
                        .zip(e.to_cols_array())
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0_f32, f32::max);
                    assert!(
                        error < 3e-6,
                        "Euler order{} t{} error{}",
                        c["order"],
                        time,
                        error
                    );
                }
                _ => panic!("unrecognized native case"),
            }
            count += 1;
        }
        assert_eq!(count, 168);
    }
    #[test]
    fn source_hq_death_rich_geometry_moves_and_runtime_stays_strict() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/animation-browser-20261017/export/hq-sample-v2/rigs/Buildings");
        for nation in ["GBR", "GER", "JPN", "RUS", "USA"] {
            let path = root.join(format!(
                "{nation}/{nation}Corps/{nation}CorpsDeath.rig.json"
            ));
            let asset = AnimatedAsset::load_preview(&path).unwrap();
            assert!(asset.validate().is_err());
            let mut first = None;
            let mut motion = 0.0_f32;
            for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let pose = asset
                    .sample_preview(0, asset.clips[0].duration * fraction, PlaybackMode::Clamp)
                    .unwrap();
                assert!(
                    pose.meshes
                        .iter()
                        .all(|m| finite(m.positions.iter().flatten().copied())
                            && finite(m.normals.iter().flatten().copied()))
                );
                if let Some(old) = &first {
                    let old: &SampledPose = old;
                    for (a, b) in old.meshes.iter().zip(&pose.meshes) {
                        for (a, b) in a
                            .positions
                            .iter()
                            .flatten()
                            .zip(b.positions.iter().flatten())
                        {
                            motion = motion.max((a - b).abs());
                        }
                    }
                } else {
                    first = Some(pose);
                }
            }
            assert!(motion > 0.01, "{nation} death was static");
        }
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    #[derive(Deserialize)]
    struct RefMesh {
        name: String,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
    }
    #[derive(Deserialize)]
    struct RefPose {
        clip: Option<usize>,
        time: f32,
        meshes: Vec<RefMesh>,
    }
    #[derive(Deserialize)]
    struct Reference {
        poses: Vec<RefPose>,
    }
    #[test]
    fn all_25_hq_death_geometry_matches_raw_nif_pyffi_reference() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/animation-browser-20261017/export");
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("references-hq25-v2/geometry-references.json")).unwrap(),
        )
        .unwrap();
        let mut count = 0;
        let mut maxp = 0.0_f32;
        let mut maxn = 0.0_f32;
        for record in manifest["records"].as_array().unwrap() {
            let a =
                AnimatedAsset::load_preview(root.join(record["rig"].as_str().unwrap())).unwrap();
            let r: Reference = serde_json::from_slice(
                &fs::read(root.join(record["reference"].as_str().unwrap())).unwrap(),
            )
            .unwrap();
            for r in r.poses {
                let pose = if let Some(c) = r.clip {
                    a.sample_preview(c, r.time, PlaybackMode::Clamp).unwrap()
                } else {
                    a.sample_bind_preview().unwrap()
                };
                assert_eq!(pose.meshes.len(), r.meshes.len());
                for ((actual, expected), source) in pose.meshes.iter().zip(r.meshes).zip(&a.meshes)
                {
                    assert_eq!(expected.name, source.name);
                    assert_eq!(actual.positions.len(), expected.positions.len());
                    assert_eq!(actual.normals.len(), expected.normals.len());
                    for (x, y) in actual
                        .positions
                        .iter()
                        .flatten()
                        .zip(expected.positions.iter().flatten())
                    {
                        maxp = maxp.max((x - y).abs());
                    }
                    for (x, y) in actual
                        .normals
                        .iter()
                        .flatten()
                        .zip(expected.normals.iter().flatten())
                    {
                        maxn = maxn.max((x - y).abs());
                    }
                }
                count += 1;
            }
        }
        println!("{count} original HQ death poses: max position {maxp}, max normal {maxn}");
        assert_eq!(count, 170);
        assert!(maxp < 0.00002, "position{maxp}");
        assert!(maxn < 0.00002, "normal{maxn}");
    }
}

#[cfg(test)]
mod edge_and_tcb_tests {
    use super::*;
    #[test]
    fn native_tcb3_scalar_and_vector_nonuniform_intervals_match() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/animation-browser-20261017/native-curves/native-tcb3.json"
        ))
        .unwrap();
        let mut samples = 0;
        for c in doc["cases"].as_array().unwrap() {
            let vector = c["kind"] == "vector_tcb3";
            let keys:Vec<_>=c["rows"].as_array().unwrap().iter().map(|r|{
    if vector{serde_json::json!({"time":r[0],"value":[r[1],r[2],r[3]],"tension":r[4],"bias":r[5],"continuity":r[6]})}
    else{serde_json::json!({"time":r[0],"value":r[1],"tension":r[2],"bias":r[3],"continuity":r[4]})}
   }).collect();
            let value = serde_json::json!({"interpolation":3,"keys":keys});
            for sample in c["native"]["samples"].as_array().unwrap() {
                let time = f32::from_bits(sample["time_word"].as_u64().unwrap() as u32);
                let out = if vector {
                    super::vector(&serde_json::from_value(value.clone()).unwrap(), time)
                        .unwrap()
                        .to_array()
                        .to_vec()
                } else {
                    vec![
                        super::scalar(&serde_json::from_value(value.clone()).unwrap(), time)
                            .unwrap(),
                    ]
                };
                for (i, v) in out.iter().enumerate() {
                    let expected =
                        f32::from_bits(sample["output_words"][i].as_u64().unwrap() as u32);
                    assert!(
                        (v - expected).abs() < 3e-5,
                        "TCB {} t{time} {v} != {expected}",
                        c["kind"]
                    );
                }
                samples += 1;
            }
        }
        assert_eq!(samples, 378);
    }
    #[test]
    fn native_empty_euler_axes_and_negative_scale_postprocessing_match() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/animation-browser-20261017/native-curves/native-curve-edges.json"
        ))
        .unwrap();
        let node = RigNode {
            name: "node".into(),
            parent: None,
            translation: [0.0; 3],
            rotation: Mat3::IDENTITY.to_cols_array_2d(),
            scale: 1.0,
        };
        for c in doc["cases"].as_array().unwrap() {
            let (rotation, scale, time) = if c["kind"] == "empty_euler" {
                let axes:Vec<_>=c["axes"].as_array().unwrap().iter().map(|axis|{let keys:Vec<_>=axis.as_array().unwrap().iter().map(|r|serde_json::json!({"time":r[0],"value":r[1],"forward":r[2],"backward":r[3]})).collect();serde_json::json!({"interpolation":2,"keys":keys})}).collect();
                (
                    serde_json::json!({"kind":"euler_xyz","channels":axes,"order":c["order"]}),
                    serde_json::json!({"interpolation":1,"keys":[]}),
                    c["time"].as_f64().unwrap() as f32,
                )
            } else {
                let v = f32::from_bits(c["input_word"].as_u64().unwrap() as u32);
                (
                    serde_json::json!({"kind":"quaternion","channel":{"interpolation":1,"keys":[]}}),
                    serde_json::json!({"interpolation":1,"keys":[{"time":0,"value":v}]}),
                    0.0,
                )
            };
            let track:NodeTrack=serde_json::from_value(serde_json::json!({"node":0,"target":"node","controller":"NiKeyframeController","frequency":1,"phase":0,"start":0,"stop":2,"flags":44,"translation":{"interpolation":1,"keys":[]},"scale":scale,"rotation":rotation})).unwrap();
            let actual = Mat3::from_mat4(local(&node, Some(&track), time).unwrap());
            if c["kind"] == "empty_euler" {
                let q = &c["native"][0];
                let q: [f32; 4] =
                    std::array::from_fn(|i| f32::from_bits(q[i].as_u64().unwrap() as u32));
                let expected = super::super::quaternion_matrix([q[1], q[2], q[3], q[0]]);
                for (a, b) in actual.to_cols_array().iter().zip(expected.to_cols_array()) {
                    assert!((a - b).abs() < 3e-6);
                }
            } else {
                let expected = f32::from_bits(c["native_word"].as_u64().unwrap() as u32);
                assert_eq!(actual.x_axis.x.to_bits(), expected.to_bits());
            }
        }
    }
    #[test]
    fn authored_alpha_is_override_and_collapsed_geometry_is_finite() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets-cache/animation/USAInfantry.rig.json");
        let mut asset = AnimatedAsset::load(path).unwrap();
        asset.clips[0].preview.alpha_tracks.push(serde_json::from_value(serde_json::json!({"meshes":[0],"target":"material","controller":"NiAlphaController","frequency":1,"phase":0,"start":0,"stop":1,"flags":44,"channel":{"interpolation":1,"keys":[{"time":0,"value":0.8},{"time":1,"value":0.2}]}})).unwrap());
        assert!(asset.sample(0, 0.5, PlaybackMode::Clamp).is_err());
        let p = asset.sample_preview(0, 0.5, PlaybackMode::Clamp).unwrap();
        assert_eq!(p.meshes[0].opacity, Some(0.5));
        assert_eq!(p.meshes[1].opacity, None);
        asset.nodes[0].scale = 0.0;
        let p = asset.sample_preview(0, 0.5, PlaybackMode::Clamp).unwrap();
        assert!(p.meshes.iter().all(|m| !m.visible
            && finite(m.positions.iter().flatten().copied())
            && finite(m.normals.iter().flatten().copied())));
    }
}

#[cfg(test)]
mod authentic_curve_tests {
    use super::*;
    #[test]
    fn actual_building_death_key_bytes_match_native_samples() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/animation-browser-20261017/native-curves/native-curves-actual.json"
        ))
        .unwrap();
        for c in f["cases"].as_array().unwrap() {
            let source = &f["sources"][c["source_index"].as_u64().unwrap() as usize];
            let channels: Vec<_> = source["axes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| super::tests::curve(a, false))
                .collect();
            let scale = super::tests::curve(&source["scales"], false);
            let track:NodeTrack=serde_json::from_value(serde_json::json!({"node":0,"target":"node","controller":"NiKeyframeController","frequency":1,"phase":0,"start":source["defined_interval"][0],"stop":source["defined_interval"][1],"flags":44,"translation":{"interpolation":1,"keys":[]},"scale":scale,"rotation":{"kind":"euler_xyz","channels":channels,"order":source["order"]}})).unwrap();
            let node = RigNode {
                name: "node".into(),
                parent: None,
                translation: [0.0; 3],
                rotation: Mat3::IDENTITY.to_cols_array_2d(),
                scale: 1.0,
            };
            let t = f32::from_bits(c["time_word"].as_u64().unwrap() as u32);
            let a = Mat3::from_mat4(local(&node, Some(&track), t).unwrap());
            let q = &c["native"]["rotation_wxyz_words"];
            let q: [f32; 4] =
                std::array::from_fn(|i| f32::from_bits(q[i].as_u64().unwrap() as u32));
            let sc = f32::from_bits(c["native"]["scale_word"].as_u64().unwrap() as u32);
            let e = super::super::quaternion_matrix([q[1], q[2], q[3], q[0]]) * sc;
            let max = a
                .to_cols_array()
                .iter()
                .zip(e.to_cols_array())
                .map(|(x, y)| (x - y).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                max < 4e-6,
                "actualcurve t{t} source{} error{max}",
                c["source_index"]
            );
        }
    }
}

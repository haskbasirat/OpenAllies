//! CPU-only authored animation catalog check; never runs gameplay or a GPU.
use aa_animation::{AnimatedAsset, PlaybackMode};
use serde_json::{Value, json};
use std::{fs, io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(args.next().ok_or("manifest path required")?);
    let out = PathBuf::from(args.next().ok_or("new output path required")?);
    let doc: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let root = path.parent().ok_or("manifest parent")?;
    let records = doc["rigs"].as_array().ok_or("rigs array")?;
    let mut checked = 0usize;
    let mut clips = 0usize;
    let mut poses = 0usize;
    let mut rows = vec![];
    let mut errors = vec![];
    let mut incomplete = 0usize;
    for r in records {
        let file = r["file"].as_str().ok_or("rig file")?;
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let a = AnimatedAsset::load_preview(root.join(file))?;
            let mut largest = 0f32;
            let mut moving = 0usize;
            let mut unsupported = 0usize;
            let mut alphatracks = 0usize;
            for (i, c) in a.clips.iter().enumerate() {
                let mut first = None;
                let mut motion = 0f32;
                for f in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let p = a.sample_preview(i, c.duration * f, PlaybackMode::Clamp)?;
                    for m in &p.meshes {
                        if !m
                            .positions
                            .iter()
                            .flatten()
                            .chain(m.normals.iter().flatten())
                            .all(|v| v.is_finite())
                            || m.opacity.is_some_and(|v| !v.is_finite())
                        {
                            return Err("nonfinite pose".into());
                        }
                    }
                    if let Some(old) = &first {
                        let old: &aa_animation::SampledPose = old;
                        for (x, y) in old.meshes.iter().zip(&p.meshes) {
                            for (x, y) in x
                                .positions
                                .iter()
                                .flatten()
                                .zip(y.positions.iter().flatten())
                            {
                                motion = motion.max((x - y).abs());
                            }
                        }
                    } else {
                        first = Some(p);
                    }
                    poses += 1;
                }
                if motion > 0.00001 {
                    moving += 1;
                }
                largest = largest.max(motion);
                unsupported += c.preview.unsupported_controllers.len();
                alphatracks += c.preview.alpha_tracks.len();
                clips += 1;
            }
            if unsupported > 0 {
                incomplete += 1;
            }
            checked += 1;
            Ok(
                json!({"source":a.source,"file":file,"clips":a.clips.len(),"sampled_poses":a.clips.len()*5,"moving_clips":moving,"maximum_vertex_displacement":largest,"alpha_tracks":alphatracks,"unsupported_controller_records":unsupported,"complete_controller_presentation":unsupported==0}),
            )
        })();
        match result {
            Ok(row) => rows.push(row),
            Err(e) => errors.push(json!({"file":file,"error":e.to_string()})),
        }
    }
    let passed = errors.is_empty();
    let report = json!({"passed":passed,"scope":"CPU-only inspector geometry/finite sampling at five authored timestamps per clip; effect metadata stays explicit. No native whole-render/gameplay parity.","manifest":path.to_string_lossy(),"catalog_rigs":records.len(),"loaded_rigs":checked,"sampled_clips":clips,"sampled_poses":poses,"rigs_with_explicit_unsupported_controllers":incomplete,"errors":errors,"rigs":rows});
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    serde_json::to_writer_pretty(&mut f, &report)?;
    writeln!(f)?;
    println!(
        "catalog{} loaded{} clips{} poses{} errors{}",
        records.len(),
        checked,
        clips,
        poses,
        errors.len()
    );
    if !passed {
        return Err("some catalog rigs refused; inspect retained errors".into());
    }
    Ok(())
}

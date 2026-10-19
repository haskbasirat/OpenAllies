//! Every shipped NIF (all `.nif` members of the original Data.rwd) against
//! the native NiStream oracle (`research/phase8-nif/native_nif_corpus.py`,
//! which runs AA.exe's own loader under Unicorn).
//!
//! Per file this compares: load success, header version and user version,
//! object count and the native class of every object, the exact sequence of
//! `NiBinaryStream::Read` calls (offset, requested and delivered length, the
//! object being loaded and the link-list lengths), the link-ID and link-count
//! lists, the LinkObject pop sequence, and the top-level objects.
//!
//! Needs the original install (Data.rwd) beside `rust-rebuild`; set
//! AA_GAME_ROOT to override. `NIF_TRACE_DUMP=<member path>` writes that file's
//! Rust trace to `target/nif-trace-dump.tsv` for `compare_trace.py`.
use aa_formats::nif::{self, Push};
use aa_formats::RwdArchive;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[path = "support/sha256.rs"]
mod sha256;

fn game_root() -> PathBuf {
    std::env::var_os("AA_GAME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."))
}

fn fixture(name: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-nif").join(name);
    let text = std::fs::read_to_string(&path).expect("native oracle fixture");
    let mut lines = text.lines();
    let cols: Vec<&str> = lines.next().unwrap().split('\t').collect();
    lines
        .map(|l| {
            let row: BTreeMap<String, String> =
                cols.iter().zip(l.split('\t')).map(|(c, v)| (c.to_string(), v.to_string())).collect();
            (row["path"].clone(), row)
        })
        .collect()
}

fn words_digest(words: &[u32]) -> String {
    let mut h = sha256::Sha256::new();
    for w in words {
        h.update(&w.to_le_bytes());
    }
    h.hex()
}

fn pops_json(file: &nif::NifFile) -> String {
    let mut out = String::from("[");
    let (mut id, mut count) = (0usize, 0usize);
    let mut first = true;
    for (object, pushes) in file.pushes.iter().enumerate() {
        for push in pushes {
            let (kind, pos, value) = match push {
                Push::Id => {
                    id += 1;
                    ("id", id - 1, file.link_ids[id - 1])
                }
                Push::Count => {
                    count += 1;
                    ("count", count - 1, file.link_counts[count - 1])
                }
            };
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!("[{object},\"{kind}\",{pos},{value}]"));
        }
    }
    out.push(']');
    out
}

#[test]
fn every_shipped_nif_matches_the_native_stream_reader() {
    check(".nif", "native-nif-corpus.tsv");
}

/// Every shipped KF (NiControllerSequence, NiTextKeyExtraData,
/// NiKeyframeController/Data; 10.1.0.0 and 10.0.1.0) against the same oracle
/// (NIF_CORPUS_EXT=.kf native_nif_corpus.py).
#[test]
fn every_shipped_kf_matches_the_native_stream_reader() {
    check(".kf", "native-kf-corpus.tsv");
}

fn check(ext: &str, name: &str) {
    let root = game_root();
    let archive_path = root.join("Data.rwd");
    if !archive_path.is_file() {
        eprintln!("skipping: {} not present", archive_path.display());
        return;
    }
    let native = fixture(name);
    let mut archive = RwdArchive::open(&archive_path).expect("Data.rwd");
    let members: Vec<(usize, String)> = archive
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| e.path().to_ascii_lowercase().ends_with(ext))
        .map(|(i, e)| (i, e.path().to_string()))
        .collect();
    assert_eq!(members.len(), native.len(), "shipped NIF member census differs from the oracle");
    let dump = std::env::var("NIF_TRACE_DUMP").ok();
    let mut failures = Vec::new();
    let mut versions: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for (index, path) in members {
        let mut data = Vec::new();
        archive.copy_entry(index, &mut data).unwrap();
        let row = native.get(&path).unwrap_or_else(|| panic!("{path} absent from the oracle"));
        assert_eq!(sha256::hex(&data), row["sha256"], "{path} bytes differ from the oracle input");
        let traced = nif::load_traced(&data);
        if dump.as_deref() == Some(path.as_str()) {
            let mut text = String::new();
            for r in &traced.trace {
                text.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\n",
                    r.offset, r.requested, r.delivered, r.object, r.link_ids, r.link_counts
                ));
            }
            std::fs::write(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/nif-trace-dump.tsv"), text).unwrap();
        }
        let mut trace = sha256::Sha256::new();
        for r in &traced.trace {
            for w in [r.offset, r.requested, r.delivered, r.object, r.link_ids, r.link_counts] {
                trace.update(&w.to_le_bytes());
            }
        }
        let trace_digest = trace.hex();
        let file = match traced.result {
            Ok(file) => file,
            Err(e) => {
                failures.push(format!("{path}: {e} (trace {} of {} reads)", traced.trace.len(), row["reads"]));
                continue;
            }
        };
        *versions.entry((file.version, file.user_version)).or_default() += 1;
        let classes: Vec<&str> = file.objects.iter().map(|o| o.class).collect();
        let roots: Vec<String> =
            file.top_level.iter().map(|r| r.map(|i| i.to_string()).unwrap_or_else(|| "n".into())).collect();
        let checks = [
            ("version", file.version.to_string(), row["version"].clone()),
            ("user_version", file.user_version.to_string(), row["user_version"].clone()),
            ("objects", file.objects.len().to_string(), row["objects"].clone()),
            ("classes", sha256::hex(classes.join("\n").as_bytes()), row["classes_sha256"].clone()),
            ("reads", traced.trace.len().to_string(), row["reads"].clone()),
            ("trace", trace_digest, row["trace_sha256"].clone()),
            ("link_ids", words_digest(&file.link_ids), row["link_ids_sha256"].clone()),
            ("link_counts", words_digest(&file.link_counts), row["link_counts_sha256"].clone()),
            ("pops", sha256::hex(pops_json(&file).as_bytes()), row["pops_sha256"].clone()),
            ("roots", roots.join(","), row["roots"].clone()),
        ];
        let bad: Vec<&str> = checks.iter().filter(|(_, a, b)| a != b).map(|(n, _, _)| *n).collect();
        if !bad.is_empty() {
            failures.push(format!("{path}: differs in {bad:?}"));
        }
        if !row["destroyed"].is_empty() && file.requires_native_post_process.is_empty() {
            failures.push(format!("{path}: native post-process converts objects but none was reported"));
        }
        if row["destroyed"].is_empty() && !file.requires_native_post_process.is_empty() {
            failures.push(format!("{path}: reported a post-process the native load did not perform"));
        }
    }
    eprintln!("versions: {versions:?}");
    if !failures.is_empty() {
        let n = failures.len();
        failures.truncate(40);
        panic!("{n} files differ from the native reader:\n{}", failures.join("\n"));
    }
}

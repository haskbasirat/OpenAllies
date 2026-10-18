//! Original animation inspector catalog. No gameplay animation selection occurs here.
use aa_assets::AssetStore;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct AnimationChoice {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    pub clips: Vec<String>,
    pub limitations: Vec<String>,
    pub aliases: Vec<String>,
    pub sha256: Option<String>,
}
impl AnimationChoice {
    pub fn matches(&self, query: &str) -> bool {
        let text = format!("{} {} {} {}", self.name, self.source, self.clips.join(" "), self.aliases.join(" ")).to_lowercase();
        query.split_whitespace().all(|word| text.contains(&word.to_lowercase()))
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct UnavailableAnimation {
    pub source: String,
    pub reason: String,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array).into_iter().flatten()
        .filter_map(Value::as_str).map(str::to_owned).collect()
}
fn relative_file(base: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['\\', ':', '\0'])
        || name.split('/').any(|part| matches!(part, "" | "." | ".."))
        || Path::new(name).components().any(|part| !matches!(part, Component::Normal(_)))
        || !name.ends_with(".rig.json") {
        bail!("Unsafe animation catalog file: {name}");
    }
    let root = base.canonicalize()?;
    let path = base.join(name).canonicalize().with_context(|| format!("Animation file missing: {name}"))?;
    if !path.starts_with(root) || !path.is_file() { bail!("Animation file escapes its catalog: {name}"); }
    Ok(path)
}
fn browser_manifest(store: &AssetStore) -> Option<PathBuf> {
    crate::animation_preview::cache_dir(store)
        .map(|dir| dir.join("browser/animation-browser-manifest.json"))
        .filter(|path| path.is_file())
}
fn read_manifest(path: &Path) -> Result<Value> {
    let document: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    if document["format"] != "aa-animation-browser-manifest-v1" { bail!("Unknown original animation browser manifest format"); }
    Ok(document)
}
fn manifest_choices(base: &Path, document: &Value) -> Result<Vec<AnimationChoice>> {
    let mut result = Vec::new();
    let mut sources = std::collections::BTreeSet::new();
    for row in document["rigs"].as_array().context("Animation catalog rigs are missing")? {
        let source = row["source"].as_str().context("Animation source is missing")?.to_owned();
        if !sources.insert(source.to_lowercase()) { bail!("Duplicate original animation model: {source}"); }
        let path = relative_file(base, row["file"].as_str().context("Animation rig filename missing")?)?;
        let mut aliases = strings(row.get("aliases"));
        if let Some(name) = row["name"].as_str() { aliases.push(name.to_owned()); }
        let name = Path::new(&source).file_stem().and_then(|name| name.to_str()).context("Animation model name missing")?.to_owned();
        let clips = row["clips"].as_array().context("Animation clip catalog missing")?.iter()
            .map(|clip| clip.as_str().or_else(|| clip["name"].as_str()).map(str::to_owned).context("Animation clip name missing"))
            .collect::<Result<Vec<_>>>()?;
        if clips.is_empty() { bail!("Published animation rig has no clips: {source}"); }
        let digest = row["sha256"].as_str().context("Animation rig digest missing")?;
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) { bail!("Invalid animation rig digest"); }
        result.push(AnimationChoice { name, path, source, clips, limitations: strings(row.get("limitations")), aliases, sha256: Some(digest.to_lowercase()) });
    }
    result.sort_by(|a,b| a.name.cmp(&b.name).then(a.source.cmp(&b.source)));
    Ok(result)
}

pub fn available(store: &AssetStore) -> Result<Vec<AnimationChoice>> {
    if let Some(path) = browser_manifest(store) {
        return manifest_choices(path.parent().unwrap(), &read_manifest(&path)?);
    }
    let Some(dir) = crate::animation_preview::cache_dir(store) else { return Ok(Vec::new()); };
    // Legacy releases published root rigs and a separate explicit game manifest.
    // Never scan phase3-staging: those are duplicate research artifacts.
    let mut files = std::fs::read_dir(&dir)?.filter_map(|entry| entry.ok()).map(|entry| entry.path())
        .filter(|path| path.is_file() && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".rig.json")))
        .collect::<Vec<_>>();
    let game = dir.join("game-rigs.json");
    if game.is_file() {
        let document: Value = serde_json::from_slice(&std::fs::read(game)?)?;
        for row in document["rigs"].as_array().context("Game rig manifest has no rigs")? {
            files.push(relative_file(&dir, row["file"].as_str().context("Game rig file missing")?)?);
        }
    }
    files.sort(); files.dedup();
    let mut result = Vec::new();
    for path in files {
        let document: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap().trim_end_matches(".rig.json").to_owned();
        let source = document["source"].as_str().context("Legacy rig source missing")?.to_owned();
        let clips = document["clips"].as_array().context("Legacy rig clips missing")?.iter().filter_map(|clip| clip["name"].as_str()).map(str::to_owned).collect();
        result.push(AnimationChoice {name, path, source, clips, limitations: strings(document.get("limitations")), aliases: vec![], sha256: None});
    }
    result.sort_by(|a,b| a.name.cmp(&b.name).then(a.path.cmp(&b.path)));
    Ok(result)
}

pub fn unavailable(store: &AssetStore) -> Result<Vec<UnavailableAnimation>> {
    let Some(path) = browser_manifest(store) else { return Ok(Vec::new()); };
    let document = read_manifest(&path)?;
    let mut result = Vec::new();
    for source in document["sources"].as_array().context("Original animation source census missing")? {
        if source["in_exported_timeline"] == true || !(source["kind"] == "kf" || source["has_embedded_controllers"] == true) { continue; }
        result.push(UnavailableAnimation {
            source: source["source"].as_str().context("Original animation source missing")?.to_owned(),
            reason: source["catalog_only_reason"].as_str().context("Unavailable animation must have an explicit reason")?.to_owned(),
        });
    }
    Ok(result)
}
pub fn resolve(store: &AssetStore, name: &str) -> Result<PathBuf> {
    let choices = available(store)?;
    let matches = choices.iter().filter(|choice| choice.name.eq_ignore_ascii_case(name)
        || choice.source.eq_ignore_ascii_case(name) || choice.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name))).collect::<Vec<_>>();
    match matches.as_slice() {
        [choice] => { verify_choice(choice)?; Ok(choice.path.clone()) },
        [] => bail!("Unknown original animation rig {name}; see Animations or --list-animations"),
        _ => bail!("Ambiguous animation rig {name}; use its full original source path"),
    }
}
fn verify_choice(choice: &AnimationChoice) -> Result<()> {
    if let Some(expected) = &choice.sha256 {
        let actual = format!("{:x}", Sha256::digest(std::fs::read(&choice.path)?));
        if actual != *expected { bail!("Original animation rig cache digest mismatch: {}", choice.source); }
    }
    Ok(())
}
pub fn verify_path(store: &AssetStore, path: &Path) -> Result<()> {
    if browser_manifest(store).is_some() {
        let canonical = path.canonicalize()?;
        let choices = available(store)?;
        let choice = choices.iter().find(|choice| choice.path == canonical).context("Animation rig is not published by this catalog")?;
        verify_choice(choice)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_includes_original_model_and_clip_names() {
        let choice = AnimationChoice {name:"GBRCorpsDeath".into(),path:PathBuf::from("rig.json"),source:"Buildings/GBR/GBRCorps/GBRCorpsDeath.nif".into(),clips:vec!["Collapse".into()],limitations:vec![],aliases:vec!["HQ death".into()],sha256:None};
        assert!(choice.matches("gbr death")); assert!(choice.matches("collapse")); assert!(choice.matches("HQ")); assert!(!choice.matches("USA death"));
    }
    #[test]
    fn catalog_rejects_escaping_paths_before_reading_files() {
        for name in ["../bad.rig.json", "/bad.rig.json", "C:/bad.rig.json", "a\\bad.rig.json", "a/./bad.rig.json", "bad.json", ""] {
            assert!(relative_file(Path::new("."), name).is_err(), "{name}");
        }
    }
}

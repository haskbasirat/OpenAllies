//! The real per-class identity/weight creation reached by5ACEC0→59CB00.
//! The user explicitly requested removal of artificial RAM/VRAM caps. Rust
//!therefore retains growable class/resource ownership and current weights,
//!but deliberately does not impose59C040's fixed native eviction budgets.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    resources::Provenance,
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
type Result<T> = std::result::Result<T, String>;

///Canonical Film class. The native virtual0/8 identifies the CRT9D0370
///KKC_FilmResource registration. No caller raw class-ID/name constructor.
pub struct FilmClass {
    ram_weight: i32,
    vram_weight: i32,
    declarations: Vec<Provenance>,
    files: Vec<(String, String)>,
}
fn native_weights(current: Option<i32>) -> Result<(i32, i32)> {
    let ram = current.unwrap_or(10);
    if ram <= 0 {
        return Err(
            "ResourceCache native weight outside proved positive finite constructor domain".into(),
        );
    }
    Ok((ram, 10))
}
impl FilmClass {
    pub fn name(&self) -> &'static str {
        "KKC_FilmResource"
    }
    pub fn ram_weight(&self) -> i32 {
        self.ram_weight
    }
    pub fn vram_weight(&self) -> i32 {
        self.vram_weight
    }
    pub fn declarations(&self) -> &[Provenance] {
        &self.declarations
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
    pub(crate) fn from_current_fresh_variables(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err("ResourceCache current profile/console variable owner is required".into());
        }
        let paths = sources
            .catalog()
            .into_iter()
            .map(|p| p.logical_path)
            .collect::<Vec<_>>();
        let mut values = Vec::new();
        let mut files = Vec::new();
        for pattern in ["AVars*.tgi", "UVars*.tgi"] {
            for path in native_enumerate(pattern, &paths) {
                let file = sources.read(&path).map_err(|e| e.to_string())?;
                if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                    return Err("ResourceCache source bytes/hash differ".into());
                }
                let doc = Document::parse_native(file.bytes, Limits::default())
                    .map_err(|e| e.to_string())?;
                files.push((file.logical_path.clone(), file.sha256.clone()));
                for item in &doc.items {
                    let Item::Node(node) = item else { continue };
                    if !doc
                        .header_name(&node.header)
                        .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                    {
                        continue;
                    }
                    for item in &node.items {
                        let Item::Property(p) = item else { continue };
                        if !doc
                            .property_name(p)
                            .is_some_and(|n| n.eq_ignore_ascii_case("ResourceCacheFilmRAMWeight"))
                        {
                            continue;
                        }
                        if p.left.len() != 2
                            || !doc
                                .text(p.left[0].span)
                                .is_some_and(|t| t.eq_ignore_ascii_case("int"))
                        {
                            return Err("ResourceCacheFilmRAMWeight actual typed Int producer/order is required".into());
                        }
                        let value=doc.property_value(p).ok_or("ResourceCache weight value absent")?.trim().parse::<i32>()
                            .map_err(|_|"ResourceCache weight expression is outside proved literal Int domain")?;
                        values.push((
                            value,
                            Provenance {
                                path: file.logical_path.clone(),
                                sha256: file.sha256.clone(),
                                span: p.span,
                            },
                        ));
                    }
                }
            }
        }
        let current=match values.as_slice() {
            //59CB73's actual missing-variable branch passes10 for both weights.
            []=>None,
            [(value,_)]=>Some(*value),
            _=>return Err("ResourceCache duplicate/reloaded current weight variable requires actual assignment owner".into()),
        };
        //The bounded native constructor/accounting proof excludes unsigned
        //division by zero/negative-weight reinterpretation. Refuse that source
        //rather than silently replacing it with the absent-variable branch.
        let (ram_weight, vram_weight) = native_weights(current)?;
        //59CBD4..59CBE0 keeps VRAM weight10 even when the RAM variable exists.
        Ok(Self {
            ram_weight,
            vram_weight,
            declarations: values.into_iter().map(|(_, p)| p).collect(),
            files,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_film_class_creation_weights_and_cached_read() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/resource-work-depot/native-work-depot.json"
        ))
        .unwrap();
        assert_eq!(doc["passed"], true);
        assert_eq!(doc["classes"].as_array().unwrap().len(), 18);
        for row in doc["classes"].as_array().unwrap() {
            let current = row["weight"].as_i64().map(|v| v as i32);
            let (ram, vram) = native_weights(current).unwrap();
            assert_eq!(ram, row["ram_weight"].as_i64().unwrap() as i32);
            assert_eq!(vram, row["vram_weight"].as_i64().unwrap() as i32);
            assert_eq!(row["ram_total"], ram);
            assert_eq!(row["vram_total"], vram);
            assert_eq!(row["count"], 1);
            assert_eq!(row["cached_lookups"], 0);
            assert_eq!(
                String::from_utf16(
                    &serde_json::from_value::<Vec<u16>>(row["name"].clone()).unwrap()
                )
                .unwrap(),
                "KKC_FilmResource"
            );
        }
        assert!(native_weights(Some(0)).is_err());
        assert!(native_weights(Some(-1)).is_err());
    }
    #[test]
    #[ignore = "requires actual AA_GAME_ROOT archive and verified v14 source files"]
    fn actual_film_class_current_ram_three_and_native_vram_ten() {
        use aa_rules::{actors::ActorCatalog, sources::SourceLimits};
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files = V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let actors = ActorCatalog::load_v14(&mut files).unwrap();
        let catalog = Catalog::new(&actors);
        assert!(
            FilmClass::from_current_fresh_variables(&catalog, &mut files, NativeOverrides::Unbound)
                .is_err()
        );
        let class = FilmClass::from_current_fresh_variables(
            &catalog,
            &mut files,
            NativeOverrides::ControlledNone,
        )
        .unwrap();
        assert_eq!(class.name(), "KKC_FilmResource");
        assert_eq!(class.ram_weight(), 3);
        assert_eq!(class.vram_weight(), 10);
        assert_eq!(class.declarations().len(), 1);
        let declaration = &class.declarations()[0];
        assert_eq!(declaration.path, "UVars.tgi");
        let actual = files.read(&declaration.path).unwrap();
        assert_eq!(actual.sha256, declaration.sha256);
        let source = std::str::from_utf8(&actual.bytes).unwrap();
        assert!(source.contains("int ResourceCacheFilmRAMWeight = 3"));
        //The separately declared VRAM0 is not read by59CB00's native branch.
        assert!(source.contains("int ResourceCacheFilmVRAMWeight = 0"));
        assert!(!class.source_files().is_empty());
    }
}

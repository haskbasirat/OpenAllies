//! Texture counterpart of the proved Film59CB00 class metadata owner.
//! Bound once at the actual default Resource publication; same growable manager.
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

///Canonical Texture class. The native virtual0/8 identifies the CRT9D06B0
///KKC_TextureResource registration. No caller raw class-ID/name constructor.
pub struct TextureClass {
    ram_weight: i32,
    vram_weight: i32,
    declarations: Vec<Provenance>,
    files: Vec<(String, String)>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_native_texture_class_weight_creation_all_pc() {
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-view-asset-backend/native-default-texture.json"
        ))
        .unwrap();
        for row in data["rows"].as_array().unwrap() {
            for case in row["native"].as_array().unwrap() {
                let current = case["ram_current"].as_i64().map(|v| v as i32);
                let (ram, vram) = native_weights(current).unwrap();
                assert_eq!(ram, case["ram_weight"]);
                assert_eq!(vram, case["vram_weight"]);
                assert_eq!(ram, case["ram_total"]);
                assert_eq!(vram, case["vram_total"]);
                assert_eq!(case["class_name"], "KKC_TextureResource");
            }
        }
        assert!(native_weights(Some(0)).is_err());
        assert!(native_weights(Some(-1)).is_err());
    }
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
impl TextureClass {
    pub fn name(&self) -> &'static str {
        "KKC_TextureResource"
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
                        if !doc.property_name(p).is_some_and(|n| {
                            n.eq_ignore_ascii_case("ResourceCacheTextureRAMWeight")
                        }) {
                            continue;
                        }
                        if p.left.len() != 2
                            || !doc
                                .text(p.left[0].span)
                                .is_some_and(|t| t.eq_ignore_ascii_case("int"))
                        {
                            return Err("ResourceCacheTextureRAMWeight actual typed Int producer/order is required".into());
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

//! Current UVars reached only by WorldCreator reset5137C0.
//! Constructed once from actual wildcard files under explicit ControlledNone
//! startup. Profile/console overrides require their own producer; calls never
//! reconstruct a value from a missing current cell.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};

pub struct State {
    width: i32,
    height: i32,
    sources: Vec<aa_rules::resources::SourceIdentity>,
    files: Vec<(String, String)>,
    default_biome: String,
    independent: i32,
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self, String> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err(
                "Creator current UVars profile/console overrides require their actual owner".into(),
            );
        }
        let paths: Vec<_> = sources
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        let mut width = None;
        let mut height = None;
        let mut files = Vec::new();
        for path in native_enumerate("UVars*.tgi", &paths) {
            let file = sources.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("Creator UVar bytes/hash differ".into());
            }
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
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
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    let slot = if name.eq_ignore_ascii_case("MapDefaultWidth") {
                        &mut width
                    } else if name.eq_ignore_ascii_case("MapDefaultHeight") {
                        &mut height
                    } else {
                        continue;
                    };
                    if slot.is_some() {
                        return Err("Creator UVar duplicate assignment ordering is unbound".into());
                    }
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|s| s.eq_ignore_ascii_case("int"))
                    {
                        return Err(
                            "Creator UVar source requires its actual Int declaration".into()
                        );
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("Creator UVar source value absent")?;
                    *slot = Some(raw.trim().parse::<i32>().map_err(
                        |_| "Creator UVar requires proven native signed-integer literal scope",
                    )?);
                }
            }
        }
        //Same native wildcard enumeration/property assignment order as the
        //source load: SVars_AA updates the prior typed current cells.
        let mut default_biome = None;
        let mut independent = None;
        for path in native_enumerate("Game/SVars*.tgi", &paths) {
            let file = sources.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("Creator SVar bytes/hash differ".into());
            }
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            files.push((file.logical_path.clone(), file.sha256.clone()));
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|n| n.eq_ignore_ascii_case("SVars"))
                {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    if !name.eq_ignore_ascii_case("MapDefaultBiome")
                        && !name.eq_ignore_ascii_case("DefaultKingdomsIndependent")
                    {
                        continue;
                    }
                    if p.left.len() != 2 {
                        return Err("Creator SVar untyped source override is unbound".into());
                    }
                    let kind = doc
                        .text(p.left[0].span)
                        .ok_or("Creator SVar source type absent")?;
                    let raw = doc
                        .property_value(p)
                        .ok_or("Creator SVar source value absent")?
                        .trim();
                    if name.eq_ignore_ascii_case("DefaultKingdomsIndependent") {
                        if !kind.eq_ignore_ascii_case("int") {
                            return Err("Creator independent-count SVar type differs".into());
                        }
                        independent = Some(raw.parse::<i32>().map_err(
                            |_| "Creator independent-count requires signed-integer literal scope",
                        )?);
                    } else {
                        if !kind.eq_ignore_ascii_case("string")
                            || !raw.is_ascii()
                            || raw.contains(['"', '\\', '&', '\0'])
                        {
                            return Err(
                                "Creator Biome SVar requires current plain ASCII literal scope"
                                    .into(),
                            );
                        }
                        default_biome = Some(raw.to_owned());
                    }
                }
            }
        }
        Ok(Self {
            width: width.ok_or("Creator MapDefaultWidth source absent")?,
            height: height.ok_or("Creator MapDefaultHeight source absent")?,
            sources: catalog.actors.sources.clone(),
            files,
            default_biome: default_biome.ok_or("Creator MapDefaultBiome source absent")?,
            independent: independent.ok_or("Creator DefaultKingdomsIndependent source absent")?,
        })
    }
    pub(crate) fn validate(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("Creator current UVars belong to another source universe".into());
        }
        Ok(())
    }
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    pub(crate) fn default_biome(&self) -> &str {
        &self.default_biome
    }
    pub(crate) fn independent_kingdoms(&self) -> i32 {
        self.independent
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
}

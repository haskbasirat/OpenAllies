//!Sole current AVarC0 StencilShadowColorMultiplier Point3. Actual401BD0
//!registration48/401880 starts[.5,.5,.5],kind5/flags0;4B3C10case8 and
//!4B32B0→4B2B60 copy actual typed or inferred Point3 source words unchanged.
use crate::{
    catalog::Catalog,
    combat_value_source_context::{self, SourceFpContext},
    definitions::Bank,
    load_view_variables::NativeOverrides,
    settings::Declaration,
};
use aa_rules::{
    actors::native_fresh_field_order,
    native::{index::native_enumerate, scalar_literal},
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
type Result<T> = std::result::Result<T, String>;
pub const NAME: &str = "StencilShadowColorMultiplier";
const CONSTRUCTOR: [u32; 3] = [0x3f000000; 3];
pub struct State {
    current: [u32; 3],
    sources: Vec<SourceIdentity>,
    context: SourceFpContext,
    files: Vec<(String, String)>,
    declarations: Vec<Declaration>,
    complete: bool,
}
pub struct Failure {
    pub state: Option<State>,
    pub error: String,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameraRenderSourceFailure")
            .field("owner_retained", &self.state.is_some())
            .field("error", &self.error)
            .finish()
    }
}
impl State {
    pub fn from_fresh_sources_retained(
        catalog: &Catalog<'_>,
        bank: &Bank,
        files: &mut V14Sources,
        scope: NativeOverrides,
    ) -> std::result::Result<Self, Failure> {
        let initial = (|| -> Result<Self> {
            catalog.require_source_owner()?;
            if !matches!(scope, NativeOverrides::ControlledNone) {
                return Err("camera render AVar profile/console owner is unbound".into());
            }
            let source = bank.combat_source()?;
            source.completed_means()?;
            if source.source_identities() != catalog.actors.sources.as_slice() {
                return Err("camera render AVar actual Bank/source differs".into());
            }
            Ok(Self {
                current: CONSTRUCTOR,
                sources: catalog.actors.sources.clone(),
                context: source.context().clone(),
                files: Vec::new(),
                declarations: Vec::new(),
                complete: false,
            })
        })();
        let mut state = initial.map_err(|error| Failure { state: None, error })?;
        match state.load(catalog, bank, files) {
            Ok(()) => Ok(state),
            Err(error) => Err(Failure {
                state: Some(state),
                error,
            }),
        }
    }
    fn load(&mut self, catalog: &Catalog<'_>, bank: &Bank, files: &mut V14Sources) -> Result<()> {
        let names = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect::<Vec<_>>();
        let ordered = native_enumerate("AVars*.tgi", &names);
        if !ordered.iter().any(|p| p.eq_ignore_ascii_case("AVars.tgi")) {
            return Err("camera render root AVars source absent".into());
        }
        for path in ordered {
            let file = files.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("camera render AVar source hash differs".into());
            }
            self.files
                .push((file.logical_path.clone(), file.sha256.clone()));
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err("camera render inherited Vars native loader is unbound".into());
                }
                let properties = node
                    .items
                    .iter()
                    .filter_map(|v| {
                        if let Item::Property(p) = v {
                            Some(p)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let names = properties
                    .iter()
                    .map(|p| doc.property_name(p).ok_or("camera AVar field name absent"))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                for index in native_fresh_field_order(&names).map_err(|e| e.to_string())? {
                    if !names[index].eq_ignore_ascii_case(NAME) {
                        continue;
                    }
                    let property = properties[index];
                    if !(1..=2).contains(&property.left.len()) {
                        return Err("camera render AVar declaration syntax unbound".into());
                    }
                    let kind = if property.left.len() == 2 {
                        doc.text(property.left[0].span)
                    } else {
                        None
                    };
                    let raw = doc
                        .property_value(property)
                        .ok_or("camera render Point3 source absent")?;
                    let source = Provenance {
                        path: file.logical_path.clone(),
                        sha256: file.sha256.clone(),
                        span: property.span,
                    };
                    let public = catalog
                        .settings
                        .as_ref()
                        .ok_or("camera render Settings source absent")?
                        .app_variables
                        .get(&NAME.to_ascii_lowercase())
                        .ok_or("camera render public source declaration absent")?;
                    let matching = public
                        .iter()
                        .filter(|row| row.source == source)
                        .collect::<Vec<_>>();
                    let [row] = matching.as_slice() else {
                        return Err("camera render public source span ambiguity".into());
                    };
                    if row.raw != raw || row.field_type.as_deref() != kind {
                        return Err(
                            "camera render public/private source type/raw/span differs".into()
                        );
                    }
                    self.current = point3(kind, raw, self.context.precision())?;
                    self.declarations.push((*row).clone());
                }
            }
        }
        self.validate_owner(catalog, bank)?;
        self.complete = true;
        Ok(())
    }
    fn validate_owner(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        if self.sources != catalog.actors.sources
            || source.source_identities() != self.sources.as_slice()
            || source.source_precision() != self.context.precision()
            || source.context().reference_startup() != self.context.reference_startup()
        {
            return Err("camera render actual source Bank/precision differs".into());
        }
        let rows = catalog
            .settings
            .as_ref()
            .ok_or("camera render Settings source absent")?
            .app_variables
            .get(&NAME.to_ascii_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if rows.len() != self.declarations.len()
            || rows.iter().any(|row| {
                !self.declarations.iter().any(|held| {
                    held.source == row.source
                        && held.field_type == row.field_type
                        && held.raw == row.raw
                })
            })
        {
            return Err("camera render current public source declaration differs".into());
        }
        let numeric = combat_value_source_context::bind_numeric(catalog, &self.context)?;
        crate::combat_source_receipt::require_reference_numeric(&numeric)
    }
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        if !self.complete {
            return Err("camera render AVar source pass incomplete".into());
        }
        self.validate_owner(catalog, bank)
    }
    pub fn current(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<[u32; 3]> {
        self.validate(catalog, bank)?;
        Ok(self.current)
    }
    pub fn assign(&mut self, catalog: &Catalog<'_>, bank: &Bank, current: [u32; 3]) -> Result<()> {
        self.validate(catalog, bank)?;
        for word in current {
            aa_numeric::Binary::from_f32_bits(word).map_err(|e| e.to_string())?;
        }
        self.current = current;
        Ok(())
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
}
fn point3(kind: Option<&str>, raw: &str, pc: aa_numeric::Precision) -> Result<[u32; 3]> {
    if kind.is_some_and(|k| !k.eq_ignore_ascii_case("point3")) {
        return Err("camera render actual/inferred Point3 declaration required".into());
    }
    let parts = raw.split(',').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err("camera render Point3 requires three source scalars".into());
    }
    let mut out = [0; 3];
    for (i, part) in parts.into_iter().enumerate() {
        out[i] = scalar_literal(part, pc).map_err(|e| e.to_string())?
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_c0_constructor_and_inferred_source_are_exact() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-render-variables.json"
        ))
        .unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(), 6);
        for row in fixture["rows"].as_array().unwrap() {
            let r = &row["registration"];
            assert_eq!(r["index"], 48);
            assert_eq!(r["name"], NAME);
            assert_eq!(r["kind"], 5);
            assert_eq!(r["flags"], 0);
            assert_eq!(r["point3"], serde_json::json!(CONSTRUCTOR));
        }
        for pc in [
            aa_numeric::Precision::Single24,
            aa_numeric::Precision::Double53,
            aa_numeric::Precision::Extended64,
        ] {
            assert_eq!(
                point3(None, "0.7,0.7,0.6", pc).unwrap(),
                [0x3f333333, 0x3f333333, 0x3f19999a]
            );
            assert!(point3(Some("float"), "0.7,0.7,0.6", pc).is_err());
            assert!(point3(None, "0,1", pc).is_err());
        }
    }
}

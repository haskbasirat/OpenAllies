//! Source-bound SVar declarations for live startup. Duplicate assignments remain
//! explicit until their native registration/override order is established.
use aa_numeric::Precision;
use aa_rules::{
    resources::Provenance,
    sources::{ResolvedFile, V14Sources},
    tgi::{Document, Item, Limits},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Source(#[from] aa_rules::sources::SourceError),
    #[error(transparent)]
    Syntax(#[from] aa_rules::tgi::ParseError),
    #[error(transparent)]
    Native(#[from] aa_rules::native::Error),
    #[error("unsupported native SVar {0}: {1}")]
    Field(String, String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, Serialize)]
pub struct Declaration {
    /// Untyped assignments need the registered native declaration before use.
    pub field_type: Option<String>,
    pub raw: String,
    pub source: Provenance,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Settings {
    pub declarations: BTreeMap<String, Vec<Declaration>>,
    /// Application variables: the [Vars] nodes of the `AVars*.tgi` files the
    /// original loads by wildcard (419C50). Only explicitly proven variables
    /// are read from here.
    pub app_variables: BTreeMap<String, Vec<Declaration>>,
}
/// The v14 `Data/AVars*.tgi` members (wildcard 9F7C38 `AVars*.tgi`).
pub const APP_VARIABLE_FILES: [&str; 5] = [
    "AVars.tgi",
    "AVars_AA.tgi",
    "AVars_ClientCadence.tgi",
    "AVars_Locale.tgi",
    "AVars_version.tgi",
];
impl Settings {
    pub fn load_v14(sources: &mut V14Sources, limits: Limits) -> Result<Self> {
        let mut out = Self::from_files(
            [
                sources.read("Game/SVars.tgi")?,
                sources.read("Game/SVars_AA.tgi")?,
            ],
            limits,
        )?;
        for path in APP_VARIABLE_FILES {
            collect(sources.read(path)?, "Vars", limits, &mut out.app_variables)?;
        }
        Ok(out)
    }
    /// The LanguageIDS string AVar (Data/AVars_Locale.tgi), read through
    /// 410CB0 by the map text loaders (MAPSTRINGS language selection).
    pub fn language_ids(&self) -> Result<(&str, &Provenance)> {
        let name = "LanguageIDS";
        let rows = self
            .app_variables
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| Error::Field(name.into(), "missing AVars assignment".into()))?;
        let [row] = rows.as_slice() else {
            return Err(Error::Field(
                name.into(),
                "multiple AVars assignments require proven native override order".into(),
            ));
        };
        if !row
            .field_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("string"))
        {
            return Err(Error::Field(name.into(), "expected string declaration".into()));
        }
        // 5BDF80: only a value whose first character is '"' is quoted, and
        // the quote ends at the next '"'.
        let value = match row.raw.strip_prefix('"') {
            Some(rest) => rest.split('"').next().unwrap_or_default(),
            None => row.raw.as_str(),
        };
        Ok((value, &row.source))
    }
    pub fn from_files(
        files: impl IntoIterator<Item = ResolvedFile>,
        limits: Limits,
    ) -> Result<Self> {
        let mut out = Self::default();
        for file in files {
            collect(file, "SVars", limits, &mut out.declarations)?;
        }
        Ok(out)
    }
    pub fn unique_fixed(&self, name: &str, precision: Precision) -> Result<(u32, &Provenance)> {
        let rows = self
            .declarations
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| Error::Field(name.into(), "missing source assignment".into()))?;
        let [value] = rows.as_slice() else {
            return Err(Error::Field(
                name.into(),
                "multiple assignments require proven native override order".into(),
            ));
        };
        if !value
            .field_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("fixed"))
        {
            return Err(Error::Field(
                name.into(),
                "expected fixed declaration".into(),
            ));
        }
        Ok((
            aa_rules::native::scalar_literal(&value.raw, precision)?,
            &value.source,
        ))
    }
    /// Original wildcard loader sorts SVars.tgi before SVars_AA.tgi; native
    /// Settings4B3C10 replaces this source-created type1/flags0 Bool variable.
    /// Scope is this proven variable, not arbitrary hardcoded SVar policies.
    pub fn unit_supply_enabled(&self) -> Result<(bool, &Provenance)> {
        self.two_file_bool("UnitSupplyEnabled")
    }
    /// Same declaration shape as UnitSupplyEnabled (Bool in SVars.tgi and
    /// SVars_AA.tgi); read by 565140 through 4B2E60(name,1), cached in B3D65C.
    pub fn allow_sell_under_siege(&self) -> Result<(bool, &Provenance)> {
        self.two_file_bool("AllowSellUnderSiege")
    }
    fn two_file_bool(&self, name: &str) -> Result<(bool, &Provenance)> {
        let rows = self
            .declarations
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| Error::Field(name.into(), "missing source assignments".into()))?;
        if rows.len() != 2 {
            return Err(Error::Field(
                name.into(),
                "expected both original/v14 source declarations".into(),
            ));
        }
        let mut selected = None;
        for path in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
            let matches: Vec<_> = rows
                .iter()
                .filter(|r| r.source.path.eq_ignore_ascii_case(path))
                .collect();
            let [row] = matches.as_slice() else {
                return Err(Error::Field(
                    name.into(),
                    "ambiguous native source file assignment".into(),
                ));
            };
            if !row
                .field_type
                .as_deref()
                .is_some_and(|t| t.eq_ignore_ascii_case("bool") || t.eq_ignore_ascii_case("flag"))
            {
                return Err(Error::Field(
                    name.into(),
                    "native source requires Bool/flag field".into(),
                ));
            }
            let value = match row.raw.to_ascii_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(Error::Field(
                        name.into(),
                        "unsupported Bool expression".into(),
                    ));
                }
            };
            selected = Some((value, &row.source));
        }
        selected.ok_or_else(|| Error::Field(name.into(), "no native Bool assignment".into()))
    }
}

fn collect(
    file: ResolvedFile,
    node_name: &str,
    limits: Limits,
    out: &mut BTreeMap<String, Vec<Declaration>>,
) -> Result<()> {
    if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
        return Err(Error::Field(
            file.logical_path,
            "source SHA256 differs from bytes".into(),
        ));
    }
    let doc = Document::parse_native(file.bytes, limits)?;
    for item in &doc.items {
        let Item::Node(node) = item else { continue };
        if !doc
            .header_name(&node.header)
            .is_some_and(|s| s.eq_ignore_ascii_case(node_name))
        {
            continue;
        }
        for item in &node.items {
            let Item::Property(p) = item else { continue };
            let Some(name) = doc.property_name(p) else {
                continue;
            };
            let Some(raw) = doc.property_value(p) else {
                continue;
            };
            if !(1..=2).contains(&p.left.len()) {
                return Err(Error::Field(
                    name.into(),
                    "unsupported declaration tokens".into(),
                ));
            }
            let field_type = if p.left.len() == 2 {
                Some(
                    doc.text(p.left[0].span)
                        .ok_or_else(|| {
                            Error::Field(name.into(), "invalid field type text".into())
                        })?
                        .to_owned(),
                )
            } else {
                None
            };
            out
                .entry(name.to_ascii_lowercase())
                .or_default()
                .push(Declaration {
                    field_type,
                    raw: raw.into(),
                    source: Provenance {
                        path: file.logical_path.clone(),
                        sha256: file.sha256.clone(),
                        span: p.span,
                    },
                });
        }
    }
    Ok(())
}

//! Current Creator+88 KingdomParameter names and glyph strings.
//! The missing localized +8 word is owned beside the SAME parameter array.
//! A key-only parsed source is not evidence of successful native fallback
//! lookup. Its unbound companion remains explicit until a proven producer or
//! 49AE00's unconditional first overwrite supplies the actual current value.
use crate::{catalog::Catalog, localization::Localization, resource_creator::Creator};
use aa_mapworld::header::WorldParameters;
use std::sync::Arc;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
struct Pair {
    id: Vec<u16>,
    key: Vec<u16>,
    text: Option<Vec<u16>>,
    unbound: Option<String>,
}

/// Native mutable companion words omitted by the key-only stream DTO. This
/// is not another WorldParameters array or a substitute Creator owner.
pub struct State {
    owner: Arc<()>,
    teams: Vec<Pair>,
    kingdoms: Vec<Pair>,
    objectives: Vec<Vec<Pair>>,
}

/// The exact current 50DC00 allocation within one Creator. Cloning a Creator
/// copies values into a different native allocation and invalidates this loan.
pub struct KingdomHandle {
    owner: Arc<()>,
    index: usize,
}

/// Read-only native localized pair. A header/film copy may copy these actual
/// current words; a serialized key alone cannot supply the text companion.
pub struct LocalizedPairRef<'a> {
    key: &'a [u16],
    text: &'a [u16],
}
impl LocalizedPairRef<'_> {
    pub fn key(&self) -> &[u16] {
        self.key
    }
    pub fn text(&self) -> &[u16] {
        self.text
    }
}
impl<'a> LocalizedPairRef<'a> {
    /// Move this temporary view into its original owner's two immutable loans.
    pub fn into_parts(self) -> (&'a [u16], &'a [u16]) {
        (self.key,self.text)
    }
    /// A loan only, for another already-owned native value-copy destination.
    /// This creates no mutable pair owner and supplies no missing text word.
    pub(crate) fn new(key: &'a [u16], text: &'a [u16]) -> Self {
        Self { key, text }
    }
}

fn prefix(value: &[u16]) -> &[u16] {
    &value[..value.iter().position(|&v| v == 0).unwrap_or(value.len())]
}

impl State {
    ///510DC0→50F4C0 constructs each fresh record with two independent empty
    ///CString allocations at+4/+8, before any key-only file Load. This narrow
    ///constructor is crate-private and refuses later assigned record values.
    pub(crate) fn from_native_constructor(parameters: &WorldParameters) -> Result<Self> {
        if parameters.version != 13
            || !parameters.teams.is_empty()
            || parameters.kingdoms.iter().any(|p| {
                !p.id.is_empty() || !p.display_key.is_empty() || p.string_0c.as_deref() != Some(&[]) || !p.localized_entries.is_empty()
            })
        {
            return Err("Creator constructor label fields differ from native50F4C0".into());
        }
        Ok(Self {
            owner: Arc::new(()),
            teams: Vec::new(),
            objectives: parameters.kingdoms.iter().map(|_|Vec::new()).collect(),
            kingdoms: parameters
                .kingdoms
                .iter()
                .map(|_| Pair {
                    id: Vec::new(),
                    key: Vec::new(),
                    text: Some(Vec::new()),
                    unbound: None,
                })
                .collect(),
        })
    }

    /// Bind only uniquely available current table values for 49AD70's
    /// key-only translation branch. A missing key reaches TextManager+5C and
    /// diagnostics; that fallback is required, never silently an empty text.
    pub(crate) fn from_parameters(
        catalog: &Catalog<'_>,
        parameters: &WorldParameters,
    ) -> Result<Self> {
        if !(11..=13).contains(&parameters.version) {
            return Err("Creator labels require the proven WorldParameters v11..13 source".into());
        }
        let bind = |id: &[u16], key: &[u16]| {
            //49ADA9..B1 initializes an empty key without consulting the table.
            let result = if key.is_empty() {Ok(Vec::new())} else {
                catalog.with_current_text(|table|table.lookup(key).map_err(|e|e.to_string())
                    .and_then(|value|value.ok_or_else(||
                        "49AD70 missing-key TextManager+5C fallback/diagnostic owner is unbound".into())))
            };
            let (text, unbound) = match result {
                Ok(text) => (Some(text), None),
                Err(reason) => (None, Some(reason)),
            };
            Pair {
                id: id.to_vec(),
                key: key.to_vec(),
                text,
                unbound,
            }
        };
        Ok(Self {
            owner: Arc::new(()),
            teams: parameters
                .teams
                .iter()
                .map(|p| bind(&p.id, &p.display_key))
                .collect(),
            objectives: parameters.kingdoms.iter().map(|p|p.localized_entries.iter().map(|key|bind(&p.id,key)).collect()).collect(),
            kingdoms: parameters
                .kingdoms
                .iter()
                .map(|p| bind(&p.id, &p.display_key))
                .collect(),
        })
    }

    /// 514240/4A4C00 copy mutable parameter values. The copy is a distinct
    /// allocation, while every known or unbound current companion is retained.
    pub(crate) fn copy_for_creator(&self) -> Self {
        Self {
            owner: Arc::new(()),
            teams: self.teams.clone(),
            kingdoms: self.kingdoms.clone(),
            objectives: self.objectives.clone(),
        }
    }

    pub(crate) fn validate(&self, parameters: &WorldParameters) -> Result<()> {
        let teams = self.teams.len() == parameters.teams.len()
            && self
                .teams
                .iter()
                .zip(&parameters.teams)
                .all(|(a, b)| a.id == b.id && a.key == b.display_key);
        let kingdoms = self.kingdoms.len() == parameters.kingdoms.len()
            && self
                .kingdoms
                .iter()
                .zip(&parameters.kingdoms)
                .all(|(a, b)| a.id == b.id && a.key == b.display_key);
        let objectives=self.objectives.len()==parameters.kingdoms.len()&&self.objectives.iter().zip(&parameters.kingdoms).all(|(a,b)|a.len()==b.localized_entries.len()&&a.iter().zip(&b.localized_entries).all(|(a,key)|a.id==b.id&&a.key==*key));
        if teams && kingdoms && objectives {
            Ok(())
        } else {
            Err("Creator label companions do not belong to the current parameter allocation".into())
        }
    }

    fn index(&self, handle: &KingdomHandle) -> Result<usize> {
        if !Arc::ptr_eq(&self.owner, &handle.owner) || handle.index >= self.kingdoms.len() {
            return Err("Creator KingdomHandle belongs to a different parameter allocation".into());
        }
        Ok(handle.index)
    }

    /// Inspect a proved current word, refusing a still unbound native value.
    pub fn current_name(&self, handle: &KingdomHandle) -> Result<&[u16]> {
        let pair = &self.kingdoms[self.index(handle)?];
        pair.text.as_deref().ok_or_else(|| {
            pair.unbound
                .clone()
                .unwrap_or_else(|| "Creator localized companion is unbound".into())
        })
    }

    pub fn team_pair(
        &self,
        parameters: &WorldParameters,
        index: usize,
    ) -> Result<LocalizedPairRef<'_>> {
        self.validate(parameters)?;
        self.pair(
            self.teams
                .get(index)
                .ok_or("Creator TeamParameter index is outside its current array")?,
        )
    }
    pub fn kingdom_pair(
        &self,
        parameters: &WorldParameters,
        index: usize,
    ) -> Result<LocalizedPairRef<'_>> {
        self.validate(parameters)?;
        self.pair(
            self.kingdoms
                .get(index)
                .ok_or("Creator KingdomParameter index is outside its current array")?,
        )
    }
    ///Current50D5F0 objective pair, produced by510580→49AD70 and copied
    ///alongside the SAME owning KingdomParameter. Dump never re-localizes it.
    pub fn objective_pair(&self,parameters:&WorldParameters,kingdom:usize,index:usize)->Result<LocalizedPairRef<'_>> {
        self.validate(parameters)?;
        self.pair(self.objectives.get(kingdom).and_then(|v|v.get(index)).ok_or("Creator objective pair index outside its actual array")?)
    }
    fn pair<'a>(&self, pair: &'a Pair) -> Result<LocalizedPairRef<'a>> {
        let text = pair.text.as_deref().ok_or_else(|| {
            pair.unbound
                .clone()
                .unwrap_or_else(|| "Creator localized companion is unbound".into())
        })?;
        Ok(LocalizedPairRef {
            key: &pair.key,
            text,
        })
    }

    fn set_name_fields(
        &mut self,
        parameters: &WorldParameters,
        kingdom: &[u16],
        supplied: &[u16],
        table: &mut Localization,
    ) -> Result<KingdomHandle> {
        self.validate(parameters)?;
        //50DC00 compares native CString pointers through _wcscmp: first
        //case-sensitive, NUL-terminated match in the actual current array.
        let index = parameters
            .kingdoms
            .iter()
            .position(|p| prefix(&p.id) == prefix(kingdom))
            .ok_or("50DC00 current Creator KingdomParameter was not found")?;
        let pair = &mut self.kingdoms[index];
        //49AE08 writes +8 BEFORE the key lookup. Unknown old data is not read.
        //This new value survives a required missing/duplicate table error.
        pair.text = Some(supplied.to_vec());
        pair.unbound = None;
        crate::live_map_strings::set_existing_pair(
            table,
            &pair.key,
            pair.text.as_mut().expect("actual first assignment"),
            supplied,
        )?;
        Ok(KingdomHandle {
            owner: Arc::clone(&self.owner),
            index,
        })
    }

    fn set_glyph_fields(
        &self,
        parameters: &mut WorldParameters,
        handle: &KingdomHandle,
        text: &[u16],
    ) -> Result<()> {
        self.validate(parameters)?;
        let index = self.index(handle)?;
        //773E30/405B80 overwrites the same record+0C CString in full, including
        //embedded NULs. A source v11 missing read is not invented as empty.
        parameters.kingdoms[index].string_0c = Some(text.to_vec());
        Ok(())
    }
}

/// Actual Creator88 name mutation over its retained pair and live table.
pub fn set_name(
    creator: &mut Creator,
    kingdom: &[u16],
    supplied: &[u16],
    table: &mut Localization,
) -> Result<KingdomHandle> {
    let (parameters, labels) = creator.label_fields_mut();
    labels.set_name_fields(parameters, kingdom, supplied, table)
}

/// Actual +0C glyph assignment using the previously found SAME current record.
pub fn set_glyph(creator: &mut Creator, handle: &KingdomHandle, text: &[u16]) -> Result<()> {
    let (parameters, labels) = creator.label_fields_mut();
    labels.set_glyph_fields(parameters, handle, text)
}

#[cfg(test)]
#[path = "creator_labels_tests.rs"]
mod tests;

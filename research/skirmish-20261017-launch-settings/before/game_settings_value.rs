//! Mutable Session+68 GameSettings value copied by native5A46D0.
//!
//! This is a successful value-copy projection, separate from both the source
//! MapResource+90 and its appended Creator+88. Unobserved non-wire source
//! words remain unbound; they are never manufactured from the stream DTO.
use crate::{
    catalog::Catalog, creator_labels, definitions::Bank, load_world_rules::CreatorRules,
    resource_creator::SourceResource,
};
use aa_mapworld::{
    SourceIdentity,
    header::{HeaderEntry, WorldParameters},
};
use std::sync::Arc;

/// Exact scalar/string part of5A46D0. The derived MapHeader+50..64 and
/// appended Creator are deliberately absent: native5A46D0 never copies them.
#[derive(Clone)]
struct Fields {
    words_0c_10: [u32; 2],
    word_14: Option<u32>,
    words_18_1c: [u32; 2],
    name_key: Vec<u16>,
    name_text: Option<Vec<u16>>,
    description_key: Vec<u16>,
    description_text: Option<Vec<u16>>,
    display_extents: Option<[u32; 2]>,
    string_38: Vec<u16>,
    string_3c: Vec<u16>,
    word_40: Option<u32>,
    entries: Vec<HeaderEntry>,
}
impl Fields {
    fn source_header(header: &aa_mapworld::header::MapHeader) -> Self {
        Self {
            words_0c_10: header.words_0c_10,
            word_14: None,
            words_18_1c: header.words_18_1c,
            name_key: header.name_key.clone(),
            name_text: None,
            description_key: header.description_key.clone(),
            description_text: None,
            display_extents: header.display_extents_bits,
            string_38: header.string_38.clone(),
            string_3c: header.string_3c.clone(),
            word_40: header.word_40,
            entries: header.entries.clone(),
        }
    }
    /// Native order after Rules/parameters: scalarC/10/14, six CStrings,
    /// word40, entries44, then time18/1C and extent30/34. All source words
    /// here are retained values. Rust allocation failure/SEH is not emulated.
    fn copy_value(&self) -> Self {
        let words_0c_10 = self.words_0c_10;
        let word_14 = self.word_14;
        let name_key = self.name_key.clone();
        let name_text = self.name_text.clone();
        let description_key = self.description_key.clone();
        let description_text = self.description_text.clone();
        let string_38 = self.string_38.clone();
        let string_3c = self.string_3c.clone();
        let word_40 = self.word_40;
        let entries = self.entries.clone();
        let words_18_1c = self.words_18_1c;
        let display_extents = self.display_extents;
        Self {
            words_0c_10,
            word_14,
            words_18_1c,
            name_key,
            name_text,
            description_key,
            description_text,
            display_extents,
            string_38,
            string_3c,
            word_40,
            entries,
        }
    }
}

/// Move-only current GameSettings allocation. No raw-array/Clone/Default
/// constructor is exposed. Future UI/order writers must mutate this owner.
pub struct Owner {
    source: SourceIdentity,
    rules: CreatorRules,
    parameters: WorldParameters,
    labels: creator_labels::State,
    modified68: Vec<u8>,
    fields: Fields,
}
impl Owner {
    /// Actual initial selected Header90 -> Session68 value projection at
    ///75B36C→76B500→5A46D0. This is not a copy of the appended source88.
    /// The copied parameter localized companions are the SAME currently bound
    /// source values; mutable Localization is not queried again here.
    pub fn copy_source_header_5a46d0(
        catalog: &Catalog<'_>,
        bank: &Bank,
        resource: &Arc<SourceResource>,
    ) -> Result<Self, String> {
        resource.parsed_creator().validate(catalog, bank)?;
        resource.header_rules().validate_in_bank(catalog, bank)?;
        resource
            .header_labels()
            .validate(&resource.header().parameters)?;
        let rules = resource.header_rules().copy_for_creator();
        let parameters = resource.header().parameters.clone();
        let labels = resource.header_labels().copy_for_creator();
        let modified68 = resource.header_modified68().to_vec();
        let fields = Fields::source_header(resource.header()).copy_value();
        Ok(Self {
            source: resource.source().clone(),
            rules,
            parameters,
            labels,
            modified68,
            fields,
        })
    }
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        self.rules.validate_in_bank(catalog, bank)?;
        self.labels.validate(&self.parameters)?;
        if self.modified68.len() != self.parameters.kingdoms.len() {
            return Err(
                "GameSettings parameter68 companions differ from actual record array".into(),
            );
        }
        Ok(())
    }
    pub fn parameters(&self) -> &WorldParameters {
        &self.parameters
    }
    pub fn rules(&self) -> &CreatorRules {
        &self.rules
    }
    pub fn labels(&self) -> &creator_labels::State {
        &self.labels
    }
    pub fn modified68(&self) -> &[u8] {
        &self.modified68
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn words_0c_10(&self) -> [u32; 2] {
        self.fields.words_0c_10
    }
    pub fn words_18_1c(&self) -> [u32; 2] {
        self.fields.words_18_1c
    }
    pub fn string_38(&self) -> &[u16] {
        &self.fields.string_38
    }
    pub fn string_3c(&self) -> &[u16] {
        &self.fields.string_3c
    }
    pub fn entries(&self) -> &[HeaderEntry] {
        &self.fields.entries
    }
    pub fn display_extents_bits(&self) -> Result<[u32; 2], String> {
        self.fields
            .display_extents
            .ok_or_else(|| "GameSettings source display extent producer is unbound".into())
    }
    pub fn word_14(&self) -> Result<u32, String> {
        self.fields
            .word_14
            .ok_or_else(|| "GameSettings source non-wire+14 producer is unbound".into())
    }
    pub fn word_40(&self) -> Result<u32, String> {
        self.fields
            .word_40
            .ok_or_else(|| "GameSettings old header+40 constructor producer is unbound".into())
    }
    pub fn name_pair(&self) -> Result<creator_labels::LocalizedPairRef<'_>, String> {
        Ok(creator_labels::LocalizedPairRef::new(
            &self.fields.name_key,
            self.fields
                .name_text
                .as_deref()
                .ok_or("GameSettings source Header20/24 localized companion producer is unbound")?,
        ))
    }
    pub fn description_pair(&self) -> Result<creator_labels::LocalizedPairRef<'_>, String> {
        Ok(creator_labels::LocalizedPairRef::new(
            &self.fields.description_key,
            self.fields
                .description_text
                .as_deref()
                .ok_or("GameSettings source Header28/2C localized companion producer is unbound")?,
        ))
    }
}

#[cfg(test)]
#[path = "game_settings_value_tests.rs"]
mod tests;

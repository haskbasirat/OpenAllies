//! Fresh film Header5A23F0(false), including5A14C0's immediate stores.
//! This owns independent Rules/Parameters and has no decoded-map identity.
//! Native allocator/CString allocation success is the admitted domain; old
//! Header reset/destruction and the optional localized-default branch are not
//! silently substituted for this fresh film caller.
use crate::{
    catalog::Catalog, creator_labels::LocalizedPairRef, definitions::Bank,
    film_header_fields::Destination, film_parameters, film_process_metadata,
    load_world_rules::CreatorRules,
};
use aa_mapworld::header::HeaderEntry;

pub type Result<T> = std::result::Result<T, String>;

struct Pair {
    key: Vec<u16>,
    text: Vec<u16>,
}
impl Pair {
    fn empty() -> Self {
        Self {
            key: Vec::new(),
            text: Vec::new(),
        }
    }
    fn loan(&self) -> LocalizedPairRef<'_> {
        LocalizedPairRef::new(&self.key, &self.text)
    }
}

/// Native scalar/CString fields, including the meaningful failed-clock
/// prefix. byte64 remains uninitialized until6FB1F0 has actually returned.
/// No raw/Default/Clone constructor is exposed.
pub struct Fields {
    words_0c_10: [u32; 2],
    word_14: u32,
    time_18_1c: [u32; 2],
    name: Pair,
    description: Pair,
    extents: [u32; 2],
    recording_38: Vec<u16>,
    campaign_3c: Vec<u16>,
    campaign_index_40: u32,
    build_50: u32,
    numeric_version_54: u32,
    version_58: Vec<u16>,
    application_5c: Vec<u16>,
    release_60: Vec<u16>,
    byte_64: Option<u8>,
    entries: Vec<HeaderEntry>,
}
impl Fields {
    pub fn words_0c_10(&self) -> [u32; 2] {
        self.words_0c_10
    }
    pub fn word_14(&self) -> u32 {
        self.word_14
    }
    pub fn local_filetime(&self) -> [u32; 2] {
        self.time_18_1c
    }
    pub fn name(&self) -> LocalizedPairRef<'_> {
        self.name.loan()
    }
    pub fn description(&self) -> LocalizedPairRef<'_> {
        self.description.loan()
    }
    pub fn extents_bits(&self) -> [u32; 2] {
        self.extents
    }
    pub fn recording_name(&self) -> &[u16] {
        &self.recording_38
    }
    pub fn campaign_name(&self) -> &[u16] {
        &self.campaign_3c
    }
    pub fn campaign_index(&self) -> u32 {
        self.campaign_index_40
    }
    pub fn build_bits(&self) -> u32 {
        self.build_50
    }
    pub fn numeric_version(&self) -> u32 {
        self.numeric_version_54
    }
    pub fn version_text(&self) -> &[u16] {
        &self.version_58
    }
    pub fn application(&self) -> &[u16] {
        &self.application_5c
    }
    pub fn release(&self) -> &[u16] {
        &self.release_60
    }
    pub fn byte_64(&self) -> Option<u8> {
        self.byte_64
    }
    pub fn entries(&self) -> &[HeaderEntry] {
        &self.entries
    }
}

/// A failure keeps the real field stores and any completed independent child.
/// In particular, process header-count increments never roll back on errors.
pub struct Failure {
    fields: Fields,
    rules: Option<CreatorRules>,
    parameters: Option<film_parameters::State>,
    error: String,
}
impl Failure {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn fields(&self) -> &Fields {
        &self.fields
    }
    pub fn rules(&self) -> Option<&CreatorRules> {
        self.rules.as_ref()
    }
    pub fn parameters(&self) -> Option<&film_parameters::State> {
        self.parameters.as_ref()
    }
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilmHeaderFailure")
            .field("error", &self.error)
            .field("rules_present", &self.rules.is_some())
            .field("parameters_present", &self.parameters.is_some())
            .finish()
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}
impl std::error::Error for Failure {}

/// Native constructed, independent mutable Header. Move this owner into the
/// actual film resource; copying a source map's Header cannot construct it.
pub struct State {
    fields: Fields,
    rules: CreatorRules,
    parameters: film_parameters::State,
}
impl State {
    ///5A31F1 supplies false through5A27A0 to5A23F0. Hence no map_name or
    ///map_description localization is performed. The current-time callback is
    ///required at the actual6FB1F0 point, after retained process metadata writes.
    pub fn from_film_constructor(
        catalog: &Catalog<'_>,
        bank: &Bank,
        process: &mut film_process_metadata::State,
        current_local_filetime: impl FnOnce() -> Result<[u32; 2]>,
    ) -> std::result::Result<Self, Failure> {
        //Native5A23F0's fresh CString/extent/entry construction, then the first
        //5A14C0 writes. Unserialized+14 and both localized words are retained.
        let mut fields = Fields {
            words_0c_10: [7, 0],
            word_14: 0,
            time_18_1c: [0, 0],
            name: Pair::empty(),
            description: Pair::empty(),
            extents: [0, 0],
            recording_38: Vec::new(),
            campaign_3c: Vec::new(),
            campaign_index_40: u32::MAX,
            build_50: 0,
            numeric_version_54: 0,
            version_58: Vec::new(),
            application_5c: Vec::new(),
            release_60: Vec::new(),
            byte_64: None,
            entries: Vec::new(),
        };
        process.header_reset_5a14c0();
        fields.build_50 = process.build_bits();
        fields.numeric_version_54 = process.numeric_version();
        fields.version_58 = process.version_text().to_vec();
        fields.release_60 = process.release().to_vec();
        fields.application_5c = process.application().to_vec();
        let time = match current_local_filetime() {
            Ok(value) => value,
            Err(error) => {
                return Err(Failure {
                    fields,
                    rules: None,
                    parameters: None,
                    error,
                });
            }
        };
        fields.time_18_1c[0] = time[0];
        fields.time_18_1c[1] = time[1];
        fields.byte_64 = Some(0);
        let rules = match CreatorRules::from_native_constructor(catalog, bank) {
            Ok(rules) => rules,
            Err(error) => {
                return Err(Failure {
                    fields,
                    rules: None,
                    parameters: None,
                    error,
                });
            }
        };
        let parameters = film_parameters::State::from_native_constructor();
        Ok(Self {
            fields,
            rules,
            parameters,
        })
    }
    pub fn fields(&self) -> &Fields {
        &self.fields
    }
    pub fn rules(&self) -> &CreatorRules {
        &self.rules
    }
    pub fn parameters(&self) -> &film_parameters::State {
        &self.parameters
    }
    ///5A31D0 stores mode3 only after the complete5A26E0 capture returned.
    pub(crate) fn mark_film_mode_5a31d0(&mut self) {
        self.rules.set_mode_from_current(3);
    }
    ///5A3250's final405B80 value-copy, after actual resource publication.
    pub(crate) fn set_recording_name_5a3250(&mut self, name: &[u16]) {
        self.fields.recording_38 = name.to_vec();
    }
}
impl Destination for State {
    type GameType = Vec<u16>;
    type Rules = CreatorRules;
    type Parameters = film_parameters::State;
    type Entries = Vec<HeaderEntry>;
    fn set_time_high(&mut self, value: u32) {
        self.fields.time_18_1c[1] = value;
    }
    fn set_time_low(&mut self, value: u32) {
        self.fields.time_18_1c[0] = value;
    }
    fn set_rule_mode(&mut self, value: u32) {
        self.rules.set_mode_from_current(value);
    }
    fn set_game_type(&mut self, value: Self::GameType) {
        self.rules.set_game_type_from_current(value);
    }
    fn rules(&mut self) -> &mut Self::Rules {
        &mut self.rules
    }
    fn set_rule_extra(&mut self, value: u32) {
        self.rules.set_changes_from_current(value);
    }
    fn parameters(&mut self) -> &mut Self::Parameters {
        &mut self.parameters
    }
    fn set_width(&mut self, value: u32) {
        self.fields.extents[0] = value;
    }
    fn set_height(&mut self, value: u32) {
        self.fields.extents[1] = value;
    }
    fn set_campaign_name_3c(&mut self, value: Vec<u16>) {
        self.fields.campaign_3c = value;
    }
    fn set_campaign_index_40(&mut self, value: u32) {
        self.fields.campaign_index_40 = value;
    }
    fn parameters_and_entries(&mut self) -> (&mut Self::Parameters, &mut Self::Entries) {
        (&mut self.parameters, &mut self.fields.entries)
    }
}

//!Shared59E7E0 writer for decoded diagnostics and the actual fresh film
//!Header owner. Field views are immutable loans; no MapHeader identity or
//!detached current parameter DTO is manufactured by the real adapter.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    film_header::{self as codec, Result, Sink},
    film_parameter_serialization::Pair,
};
use aa_mapworld::header::{HeaderEntry, MapHeader};
pub struct Fields<'a> {
    pub words_0c_10: [u32; 2],
    pub name: Pair<'a>,
    pub description: Pair<'a>,
    pub extents: Option<[u32; 2]>,
    pub build: u32,
    pub numeric_version: u32,
    pub version: &'a [u16],
    pub application: &'a [u16],
    pub release: &'a [u16],
    pub byte64: Option<u8>,
    pub recording: &'a [u16],
    pub campaign: &'a [u16],
    pub campaign_index: Option<u32>,
    pub filetime: [u32; 2],
    pub entries: &'a [HeaderEntry],
}
pub trait HeaderSource {
    fn write_rules(&self, sink: &mut impl Sink) -> Result<()>;
    fn write_parameters(&self, sink: &mut impl Sink) -> Result<()>;
    fn fields(&self) -> Fields<'_>;
}
fn pair(sink: &mut impl Sink, p: Pair<'_>) -> Result<()> {
    codec::string(sink, p.key)?;
    sink.register_localized_pair(p.key, p.text)
}
pub fn write(sink: &mut impl Sink, source: &impl HeaderSource) -> Result<()> {
    codec::word(sink, 7)?;
    source.write_rules(sink)?;
    source.write_parameters(sink)?;
    let f = source.fields();
    for v in f.words_0c_10 {
        codec::word(sink, v)?;
    }
    pair(sink, f.name)?;
    pair(sink, f.description)?;
    for v in f
        .extents
        .ok_or("film current display extents owner missing")?
    {
        codec::word(sink, v)?;
    }
    codec::word(sink, f.build)?;
    codec::word(sink, f.numeric_version)?;
    codec::string(sink, f.version)?;
    codec::string(sink, f.application)?;
    codec::string(sink, f.release)?;
    codec::byte(
        sink,
        f.byte64.ok_or("film current header byte64 owner missing")?,
    )?;
    codec::string(sink, f.recording)?;
    codec::string(sink, f.campaign)?;
    codec::word(
        sink,
        f.campaign_index
            .ok_or("film current header+40 owner missing")?,
    )?;
    for v in f.filetime {
        codec::word(sink, v)?;
    }
    for entry in f.entries {
        codec::byte(sink, 1)?;
        codec::byte(sink, entry.flag)?;
        codec::string(sink, &entry.string_20)?;
        codec::string(sink, &entry.string_18)?;
        codec::byte(
            sink,
            entry
                .flag_v5
                .ok_or("film current entry local-player flag missing")?,
        )?;
    }
    codec::byte(sink, 0)
}
pub struct Decoded<'a>(pub &'a MapHeader);
impl HeaderSource for Decoded<'_> {
    fn write_rules(&self, sink: &mut impl Sink) -> Result<()> {
        codec::rules(sink, &self.0.rules)
    }
    fn write_parameters(&self, sink: &mut impl Sink) -> Result<()> {
        codec::parameters(sink, &self.0.parameters)
    }
    fn fields(&self) -> Fields<'_> {
        let h = self.0;
        Fields {
            words_0c_10: h.words_0c_10,
            name: Pair {
                key: &h.name_key,
                text: None,
            },
            description: Pair {
                key: &h.description_key,
                text: None,
            },
            extents: h.display_extents_bits,
            build: h.word_50,
            numeric_version: h.word_54,
            version: &h.string_58,
            application: &h.string_5c,
            release: &h.string_60,
            byte64: Some(h.byte_64),
            recording: &h.string_38,
            campaign: &h.string_3c,
            campaign_index: h.word_40,
            filetime: h.words_18_1c,
            entries: &h.entries,
        }
    }
}
struct Current<'a, 'source> {
    header: &'a crate::film_header_owner::State,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
}
impl HeaderSource for Current<'_, '_> {
    fn write_rules(&self, sink: &mut impl Sink) -> Result<()> {
        self.header
            .rules()
            .validate_in_bank(self.catalog, self.bank)?;
        //This read-only wire projection contains no localized words or mutable
        //authority. It never replaces the independent retained Rules owner.
        codec::rules(sink, &self.header.rules().to_serialized())
    }
    fn write_parameters(&self, sink: &mut impl Sink) -> Result<()> {
        crate::film_parameter_serialization::write_current(
            sink,
            self.header.parameters(),
            self.catalog,
            self.bank,
        )
    }
    fn fields(&self) -> Fields<'_> {
        let h = self.header.fields();
        let (nk, nt) = h.name().into_parts();
        let (dk, dt) = h.description().into_parts();
        Fields {
            words_0c_10: h.words_0c_10(),
            name: Pair {
                key: nk,
                text: Some(nt),
            },
            description: Pair {
                key: dk,
                text: Some(dt),
            },
            extents: Some(h.extents_bits()),
            build: h.build_bits(),
            numeric_version: h.numeric_version(),
            version: h.version_text(),
            application: h.application(),
            release: h.release(),
            byte64: h.byte_64(),
            recording: h.recording_name(),
            campaign: h.campaign_name(),
            campaign_index: Some(h.campaign_index()),
            filetime: h.local_filetime(),
            entries: h.entries(),
        }
    }
}
///Serialization only. Constructing/capturing the Header, setting native mode3,
///actual5A3250 recording-name override and depot/open remain their real caller's
///responsibility; successful byte writing does not certify those producers.
pub fn write_current(
    sink: &mut impl Sink,
    header: &crate::film_header_owner::State,
    catalog: &Catalog<'_>,
    bank: &Bank,
) -> Result<()> {
    write(
        sink,
        &Current {
            header,
            catalog,
            bank,
        },
    )
}

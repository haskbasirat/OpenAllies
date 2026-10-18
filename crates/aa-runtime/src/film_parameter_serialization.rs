//!One50D730 parameter writer shared by explicit decoded diagnostics and the
//!real captured film State. Actual pairs retain BOTH words at49AA30/5A1120;
//!no detached parameter DTO is installed or copied back into a current owner.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    film_header::{self as codec, Result, Sink},
    film_parameters::State,
};
use aa_mapworld::header::WorldParameters;
pub struct Pair<'a> {
    pub key: &'a [u16],
    pub text: Option<&'a [u16]>,
}
pub struct Team<'a> {
    pub id: &'a [u16],
    pub display: Pair<'a>,
}
pub struct Kingdom<'a> {
    pub id: &'a [u16],
    pub minor: u8,
    pub refs: [&'a [u16]; 4],
    pub display: Pair<'a>,
    pub glyph: Option<&'a [u16]>,
    pub profile: &'a [u16],
    pub allow: u8,
    pub eligible: u8,
    pub team: &'a [u16],
    pub entries: Vec<Pair<'a>>,
    pub definitions: Vec<&'a [u16]>,
    pub string58: &'a [u16],
    pub properties: Vec<&'a [u16]>,
    pub byte31: u8,
    pub byte32: u8,
}
///Read-only serialization input. An implementation does not grant authority
///to populate a real parameter State. Resource names are read at4CD050's
///reached element, after the serialized count and all preceding writes.
pub trait ParameterSource {
    fn team_count(&self) -> usize;
    fn team(&self, index: usize) -> Result<Team<'_>>;
    fn kingdom_count(&self) -> usize;
    fn kingdom(&self, index: usize) -> Result<Kingdom<'_>>;
    fn resource_count(&self, kingdom: usize) -> Result<usize>;
    fn resource(&self, kingdom: usize, index: usize) -> Result<(Vec<u16>, u32)>;
}
fn localized(sink: &mut impl Sink, pair: Pair<'_>) -> Result<()> {
    codec::string(sink, pair.key)?;
    sink.register_localized_pair(pair.key, pair.text)
}
pub fn write(sink: &mut impl Sink, source: &impl ParameterSource) -> Result<()> {
    codec::word(sink, 13)?;
    codec::count(sink, source.team_count())?;
    for i in 0..source.team_count() {
        let team = source.team(i)?;
        codec::string(sink, team.id)?;
        localized(sink, team.display)?;
    }
    codec::count(sink, source.kingdom_count())?;
    for i in 0..source.kingdom_count() {
        let k = source.kingdom(i)?;
        codec::string(sink, k.id)?;
        codec::byte(sink, k.minor)?;
        for id in k.refs {
            codec::string(sink, id)?;
        }
        localized(sink, k.display)?;
        codec::string(
            sink,
            k.glyph
                .ok_or("current film KingdomParameter+0C constructor owner missing")?,
        )?;
        codec::string(sink, k.profile)?;
        codec::byte(sink, k.allow)?;
        codec::byte(sink, k.eligible)?;
        codec::string(sink, k.team)?;
        codec::count(sink, k.entries.len())?;
        for entry in k.entries {
            localized(sink, entry)?;
        }
        codec::count(sink, k.definitions.len())?;
        for definition in k.definitions {
            codec::string(sink, definition)?;
        }
        codec::string(sink, k.string58)?;
        codec::count(sink, k.properties.len())?;
        for property in k.properties {
            codec::string(sink, property)?;
        }
        codec::byte(sink, k.byte31)?;
        codec::byte(sink, k.byte32)?;
        codec::count(sink, source.resource_count(i)?)?;
        for j in 0..source.resource_count(i)? {
            let (id, bits) = source.resource(i, j)?;
            codec::string(sink, &id)?;
            codec::word(sink, bits)?;
        }
    }
    Ok(())
}
pub struct Decoded<'a>(pub &'a WorldParameters);
impl ParameterSource for Decoded<'_> {
    fn team_count(&self) -> usize {
        self.0.teams.len()
    }
    fn team(&self, i: usize) -> Result<Team<'_>> {
        let t = self.0.teams.get(i).ok_or("decoded Team index absent")?;
        Ok(Team {
            id: &t.id,
            display: Pair {
                key: &t.display_key,
                text: None,
            },
        })
    }
    fn kingdom_count(&self) -> usize {
        self.0.kingdoms.len()
    }
    fn kingdom(&self, i: usize) -> Result<Kingdom<'_>> {
        let k = self
            .0
            .kingdoms
            .get(i)
            .ok_or("decoded Kingdom index absent")?;
        Ok(Kingdom {
            id: &k.id,
            minor: k.byte_10,
            refs: [&k.nation, &k.faction, &k.color, &k.handicap],
            display: Pair {
                key: &k.display_key,
                text: None,
            },
            glyph: k.string_0c.as_deref(),
            profile: &k.string_2c,
            allow: k.byte_29,
            eligible: k.byte_28,
            team: &k.string_14,
            entries: k
                .localized_entries
                .iter()
                .map(|key| Pair { key, text: None })
                .collect(),
            definitions: k.definitions.iter().map(Vec::as_slice).collect(),
            string58: &k.string_58,
            properties: k.properties.iter().map(Vec::as_slice).collect(),
            byte31: k.byte_31,
            byte32: k.byte_32,
        })
    }
    fn resource_count(&self, k: usize) -> Result<usize> {
        Ok(self
            .0
            .kingdoms
            .get(k)
            .ok_or("decoded Kingdom index absent")?
            .resources
            .len())
    }
    fn resource(&self, k: usize, i: usize) -> Result<(Vec<u16>, u32)> {
        self.0
            .kingdoms
            .get(k)
            .and_then(|k| k.resources.get(i))
            .cloned()
            .ok_or("decoded Resource index absent".into())
    }
}
struct Current<'a, 'source> {
    state: &'a State,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
}
impl Current<'_, '_> {
    fn row(
        &self,
        i: usize,
    ) -> Result<&crate::film_kingdom_parameters::Captured<crate::film_kingdom_parameters::Definition>>
    {
        self.state
            .kingdoms()
            .nth(i)
            .ok_or("current captured Kingdom index absent".into())
    }
    fn source_precision(&self) -> Result<aa_numeric::Precision> {
        self.catalog.require_source_owner()?;
        let source = self.bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != self.catalog.actors.sources {
            return Err("current film serializer source Bank/catalog differ".into());
        }
        Ok(source.source_precision())
    }
    fn definition<'a>(
        &self,
        d: &'a Option<crate::film_kingdom_parameters::Definition>,
    ) -> Result<&'a [u16]> {
        let Some(d) = d else {
            return Ok(&[]);
        };
        self.source_precision()?;
        let registry = self
            .catalog
            .actors
            .native_data_registry(d.domain())
            .ok_or("film captured typed registry absent")?;
        let mut matches = registry
            .entries
            .iter()
            .filter(|e| e.ids.encode_utf16().eq(d.ids().iter().copied()));
        let entry = matches
            .next()
            .ok_or("film captured definition no longer has the same immutable source")?;
        if matches.next().is_some() || &entry.source != d.source() || !entry.reloads.is_empty() {
            return Err("film captured definition source identity differs".into());
        }
        Ok(d.ids())
    }
}
impl ParameterSource for Current<'_, '_> {
    fn team_count(&self) -> usize {
        self.state.teams().len()
    }
    fn team(&self, i: usize) -> Result<Team<'_>> {
        let t = self
            .state
            .teams()
            .nth(i)
            .ok_or("current captured Team index absent")?;
        Ok(Team {
            id: t.id(),
            display: Pair {
                key: t.display_key(),
                text: Some(t.display_text()),
            },
        })
    }
    fn kingdom_count(&self) -> usize {
        self.state.kingdom_count()
    }
    fn kingdom(&self, i: usize) -> Result<Kingdom<'_>> {
        let k = self.row(i)?;
        Ok(Kingdom {
            id: &k.id,
            minor: k.minor,
            refs: [
                self.definition(&k.nation)?,
                self.definition(&k.faction)?,
                self.definition(&k.color)?,
                self.definition(&k.handicap)?,
            ],
            display: Pair {
                key: &k.key,
                text: Some(&k.text),
            },
            glyph: Some(&k.glyph),
            profile: &k.profile,
            allow: k.allow,
            eligible: k.eligible,
            team: &k.team,
            entries: k
                .localized_entries
                .iter()
                .map(|(key, text)| Pair {
                    key,
                    text: Some(text),
                })
                .collect(),
            definitions: k.definitions.iter().map(|d| d.ids()).collect(),
            string58: &k.string58,
            properties: k.properties.iter().map(Vec::as_slice).collect(),
            byte31: k.byte31,
            byte32: k.byte32,
        })
    }
    fn resource_count(&self, k: usize) -> Result<usize> {
        Ok(self.row(k)?.resource_bits.len())
    }
    fn resource(&self, k: usize, i: usize) -> Result<(Vec<u16>, u32)> {
        let bits = *self
            .row(k)?
            .resource_bits
            .get(i)
            .ok_or("captured Resource cell absent")?;
        let registry = self
            .catalog
            .actors
            .native_resource_registry(self.source_precision()?)
            .map_err(|e| e.to_string())?;
        let id = registry
            .ids
            .get(i)
            .ok_or("actual Resource registry slot absent at4CD050")?;
        Ok((id.encode_utf16().collect(), bits))
    }
}
///Actual captured State writer. It borrows the whole same owner and reads
///immutable source IDs; no cloned parameter DTO loses localized text words.
pub fn write_current(
    sink: &mut impl Sink,
    state: &State,
    catalog: &Catalog<'_>,
    bank: &Bank,
) -> Result<()> {
    write(
        sink,
        &Current {
            state,
            catalog,
            bank,
        },
    )
}

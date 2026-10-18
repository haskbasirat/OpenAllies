//! GWorld::Load's MISC_DATA body `4FD017..4FD3B3` (inside `4FC600`).
//! The caller owns chunk selection: versions18+ seek MISC_DATA; older formats
//! continue the existing stream. The original input version, not raised +2C,
//! controls every gate below. This module does not publish a ready world.
use aa_actors::organization_state::{Arg, Cursor, LoadError, Message};
use std::collections::VecDeque;

pub type Result<T> = std::result::Result<T, LoadError>;

#[derive(Debug, PartialEq, Eq)]
pub enum Reference {
    /// GWorld constructor `4F7D00` does not initialize +290.
    Uninitialized,
    Null,
    /// A token in the same authoritative definition Bank, never a GTID.
    Definition(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub struct NameListCounter {
    pub definition: u32,
    pub value: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pair {
    /// A new native allocation has no initialized value before its read.
    pub word_00: Option<u32>,
    pub word_04: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    /// +268/+26C: appends in file order; duplicate definitions are retained.
    pub name_list_counters: Vec<NameListCounter>,
    /// +278: pre-cutscene game speed (constructor reads SVarManager+114).
    pub pre_cutscene_speed: u32,
    pub weather: Reference,
    /// +294/+298, head to tail. New saved entries are prepended individually.
    pub used_strings: VecDeque<Vec<u16>>,
    /// +27C; presence=false leaves the old allocation and both old words.
    pub pair_27c: Option<Pair>,
}

/// Source-aware services, bound to the actual source-load precision and shared
/// definition Bank. Lookup receives the NUL-terminated prefix and includes
/// native alias translation; no string token
/// may be allocated for an unknown reference.
pub trait World {
    /// GGameData+59C NameList registry `463FA0`.
    fn name_list(&mut self, ids: &[u16]) -> Result<Option<u32>>;
    /// Native5A1040 warning, which continues the load when it returns.
    fn warn_unknown_name_list(&mut self, ids: &[u16]) -> Result<()>;
    /// WeatherData registry `48D500` inside `4F9280`.
    fn weather(&mut self, ids: &[u16]) -> Result<Option<u32>>;
}

impl State {
    /// The fields of ctor4F7D00 reached by this loader. No Default is provided.
    /// Storage capacities are growable rather than native inline capacity5.
    pub fn new(svar_manager_114: u32) -> Self {
        Self {
            name_list_counters: Vec::new(),
            pre_cutscene_speed: svar_manager_114,
            weather: Reference::Uninitialized,
            used_strings: VecDeque::new(),
            pair_27c: None,
        }
    }

    /// Allocation-success domain of the native path. Every preceding state
    /// mutation remains committed when a later read, lookup or warning fails.
    pub fn load(&mut self, bytes: &[u8], input_version: u32, world: &mut impl World) -> Result<usize> {
        let mut cursor=Cursor::new(bytes);
        self.load_cursor(&mut cursor,input_version,world)?;
        Ok(cursor.position())
    }

    pub fn load_cursor(&mut self,c:&mut Cursor<'_>,input_version:u32,world:&mut impl World)->Result<()> {
        if input_version>28 {
            return Err(LoadError::Unsupported("MISC_DATA requires validated GWorld input version0..28".into()));
        }
        if input_version>=12 {
            let count=c.i32()?;
            for _ in 0..count.max(0) {
                let ids=c.string()?;
                let value=c.u32()?;
                let prefix=c_string(&ids);
                if let Some(definition)=world.name_list(prefix)? {
                    self.name_list_counters.push(NameListCounter{definition,value});
                } else {
                    world.warn_unknown_name_list(prefix)?;
                }
            }
        }
        if input_version>=13 {
            self.pre_cutscene_speed=c.u32()?;
        }
        if input_version>=14 {
            let ids=c.string()?;
            if ids.is_empty() {
                self.weather=Reference::Null;
            } else {
                let prefix=c_string(&ids);
                let definition=world.weather(prefix)?;
                self.weather=definition.map_or(Reference::Null,Reference::Definition);
                if definition.is_none() {
                    let display=String::from_utf16(prefix).map_err(|_|LoadError::Unsupported(
                        "Weather unknown-IDS diagnostic contains unpaired UTF16".into()))?;
                    return Err(LoadError::Exception(Message::new(
                        "Unknown IDS %s for class %hs",
                        vec![Arg::Text(display),Arg::Text("KKC_WeatherData".into())],
                    )));
                }
            }
        }
        if input_version>=24 {
            let count=c.i32()?;
            for _ in 0..count.max(0) {
                self.used_strings.push_front(c.string()?);
            }
        }
        if input_version>=28 && c.u8()?!=0 {
            // Allocation/replacement happens before either four-byte read.
            self.pair_27c=Some(Pair{word_00:None,word_04:None});
            let pair=self.pair_27c.as_mut().unwrap();
            pair.word_00=Some(c.u32()?);
            pair.word_04=Some(c.u32()?);
        }
        Ok(())
    }
}

fn c_string(units:&[u16])->&[u16] {
    &units[..units.iter().position(|&unit|unit==0).unwrap_or(units.len())]
}

/// Constructor binding for the controlled fresh-source load window: no
/// intervening SVar mutation may have occurred. A later/UI-owned setting must
/// instead supply its actual current word to [`State::new`]. All three rule
/// views are checked; a GameSpeed override is explicitly outside this path.
pub fn from_source_constructor(
    catalog:&crate::catalog::Catalog<'_>,
    inventory:&aa_mapworld::WorldInventory,
)->std::result::Result<State,String> {
    catalog.require_source_owner()?;
    let source_pc={
        let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
        let source=bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities()!=catalog.actors.sources {
            return Err("MISC constructor source Bank/catalog identities differ".into());
        }
        source.source_precision()
    };
    let registry=catalog.actors.native_data_registry("WorldRule")
        .ok_or("MISC constructor requires registered WorldRule owner")?;
    if registry.find("GameSpeed").is_some() {
        return Err("MISC GameSpeed WorldRule default application is unbound".into());
    }
    let header=inventory.header.as_ref().ok_or("MISC constructor requires actual MAP_HEADER")?;
    let standalone:Vec<_>=inventory.opaque_initialization.iter()
        .filter(|s|s.name.eq_ignore_ascii_case("RULES")).collect();
    let [section]=standalone.as_slice() else {
        return Err("MISC constructor requires exactly one retained RULES chunk".into());
    };
    if section.source.bytes!=section.bytes.len() as u64
        || section.source.sha256!=aa_rules::sources::sha256(&section.bytes) {
        return Err("MISC constructor RULES identity differs".into());
    }
    let rules=aa_mapworld::header::WorldRules::decode_bytes(&section.bytes).map_err(|e|e.to_string())?;
    for view in [&header.rules,&header.creator.rules,&rules] {
        if view.mode!=0 {return Err("MISC GameSpeed requires proven WorldRules mode0".into());}
        for rule in &view.rules {
            let name=String::from_utf16(c_string(&rule.key)).map_err(|_|"MISC WorldRule key has unsupported UTF16")?;
            if name.eq_ignore_ascii_case("GameSpeed") {
                return Err("MISC GameSpeed saved WorldRule application is unbound".into());
            }
        }
    }
    Ok(State::new(source_reset_speed(catalog)?))
}

///Actual fresh registered SVar+114 after reset and BEFORE4FC600 publishes any
///map Rules. Serialized unknown WorldRule names cannot change this earlier
///constructor read. The older inventory diagnostic above remains separate.
pub(crate) fn source_reset_speed(catalog:&crate::catalog::Catalog<'_>)->std::result::Result<u32,String> {
    catalog.require_source_owner()?;
    let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
    let state=bank.combat_source()?;state.completed_means()?;
    if state.source_identities()!=catalog.actors.sources {return Err("GameSpeed source Bank/Catalog differ".into())}
    let source_pc=state.source_precision();
    if catalog.actors.native_data_registry("WorldRule").ok_or("GameSpeed WorldRule registry absent")?.find("GameSpeed").is_some() {
        return Err("GameSpeed registered WorldRule default reset requires its real setter".into());
    }
    let settings=catalog.settings.as_ref().ok_or("MISC constructor Settings owner is unavailable")?;
    let (bits,provenance)=settings.unique_fixed("GameSpeed",source_pc).map_err(|e|e.to_string())?;
    if !catalog.actors.sources.iter().any(|file|
        file.path.eq_ignore_ascii_case(&provenance.path)&&file.sha256==provenance.sha256) {
        return Err("MISC GameSpeed Settings source identity differs".into());
    }
    // Check the actual registered source, not merely a public Settings row's
    // claimed hash. This rejects forged raw values and spans.
    let mut matches=0usize;
    for node in catalog.actors.resolved_definitions("SVars").map_err(|e|e.to_string())? {
        if let Some(aa_rules::native::TypedValue::Scalar(value))=
            node.field("GameSpeed",source_pc).map_err(|e|e.to_string())?
            && value.source==*provenance
            && value.field_type==aa_rules::native::ScalarType::Fixed
            && value.bits==bits {
                matches+=1;
            }
    }
    if matches!=1 {return Err("MISC GameSpeed Settings value/span differs from registered source".into());}
    Ok(bits)
}

#[cfg(test)]
mod constructor_speed_tests {
    #[test]
    fn original_world_constructor_and_reset_copy_current_speed_before_rules() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/development10/native-world-constructor-speed.json")).unwrap();
        assert_eq!(fixture["native_runs"],108);
        for row in fixture["rows"].as_array().unwrap() {
            let state=super::State::new(row["input"].as_u64().unwrap() as u32);
            assert_eq!(u64::from(state.pre_cutscene_speed),row["output"].as_u64().unwrap());
        }
    }
}

//! Native TRIGGERS records as read by TriggerManager 54D3D0: manager versions 4
//! (developer maps: a trailing string and word read and discarded) and 5,
//! Trigger version 4, Element versions 2 and 3 (5468A0: same layout; 544190
//! upgrade rules are explicit errors) and Parameter version 4. Other versions
//! fail explicitly. Localization mode is supplied by the native stream context;
//! it cannot be inferred from an arbitrary byte prefix.
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("trigger input at byte {offset}: {message}")]
    Format { offset: usize, message: String },
    #[error("trigger input requires {needed} bytes; caller decoding budget is {budget}")]
    Budget { needed: usize, budget: usize },
    #[error("trigger allocation failed")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub offset: usize,
    pub length: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    Empty,
    Word(u32),
    Position([u32; 3]),
    Text(Vec<u16>),
    Localized {
        key: Vec<u16>,
        inline: Option<Vec<u16>>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub source: Span,
    pub argument: String,
    pub item: String,
    pub item_index: u32,
    pub storage: u32,
    /// Native Parameter+30 byte; retained exactly, not coerced to a bool.
    pub flag: u8,
    pub value: Value,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    pub source: Span,
    pub name: String,
    pub opcode: u32,
    pub word_10: u32,
    pub parameters: Vec<Parameter>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub source: Span,
    pub id: u32,
    pub mode: u32,
    pub enabled: u8,
    pub repeat: u8,
    pub activation_bits: u32,
    pub name: Vec<u16>,
    pub description: Vec<u16>,
    pub word_2c: u32,
    pub events: Vec<Element>,
    pub conditions: Vec<Element>,
    pub actions: Vec<Element>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub name: Vec<u16>,
    pub bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedManager {
    pub next_id: u32,
    pub triggers: Vec<Trigger>,
    pub variables: Vec<Variable>,
    pub strings: Vec<Vec<u16>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Localization {
    /// Stream+48 true and native59E520 false; original RMP external MAPSTRINGS.
    External,
    /// Native49AD70 reads a second string only when the key is nonempty.
    Inline,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    localization: Localization,
}
impl Reader<'_> {
    fn bad(&self, message: impl Into<String>) -> Error {
        Error::Format {
            offset: self.at,
            message: message.into(),
        }
    }
    fn read(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .at
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| self.bad("truncated record"))?;
        let bytes = &self.bytes[self.at..end];
        self.at = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read(4)?.try_into().unwrap()))
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.read(1)?[0])
    }
    fn text(&mut self) -> Result<Vec<u16>> {
        let count = u16::from_le_bytes(self.read(2)?.try_into().unwrap()) as usize;
        let raw = self.read(count * 2)?;
        let mut out = Vec::new();
        out.try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        out.extend(
            raw.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        );
        Ok(out)
    }
    fn identifier(&mut self) -> Result<String> {
        let units = self.text()?;
        String::from_utf16(&units).map_err(|_| self.bad("invalid UTF16 definition identifier"))
    }
    fn version(&mut self, expected: u32, kind: &str) -> Result<()> {
        let actual = self.word()?;
        if actual != expected {
            return Err(self.bad(format!(
                "unsupported {kind} version {actual}; expected {expected}"
            )));
        }
        Ok(())
    }
    fn count(&mut self, minimum_record: usize) -> Result<usize> {
        let count = self.word()? as usize;
        if count > (self.bytes.len() - self.at) / minimum_record {
            return Err(self.bad("record count exceeds remaining bytes"));
        }
        Ok(count)
    }
    fn span(&self, begin: usize) -> Span {
        Span {
            offset: begin,
            length: self.at - begin,
        }
    }
    fn parameter(&mut self) -> Result<Parameter> {
        let begin = self.at;
        self.version(4, "parameter")?;
        let argument = self.identifier()?;
        let argument = if argument == "ma_ui_button" {
            "ma_script_command".into()
        } else {
            argument
        };
        if crate::registrations::argument(&argument).is_none() {
            return Err(self.bad(format!("unknown argument {argument}")));
        }
        let item = self.identifier()?;
        let item = if item.eq_ignore_ascii_case("mi_actor_data") {
            "mi_actor_data_named".into()
        } else if item == "mi_ui_button" {
            "mi_script_command".into()
        } else {
            item
        };
        let (item_index, storage) = crate::registrations::item(&item)
            .ok_or_else(|| self.bad(format!("unknown item {item}")))?;
        // Native can choose an argument-specific default for mi_invalid. Its
        // default resolution is not silently guessed at this boundary.
        if item_index == 0 {
            return Err(self.bad("mi_invalid requires native argument-default resolution"));
        }
        let flag = self.byte()?;
        let value = match storage {
            0 => Value::Empty,
            1 | 2 => Value::Word(self.word()?),
            5 => Value::Position([self.word()?, self.word()?, self.word()?]),
            6 => Value::Text(self.text()?),
            7 => {
                let key = self.text()?;
                let inline = if !key.is_empty() && self.localization == Localization::Inline {
                    Some(self.text()?)
                } else {
                    None
                };
                Value::Localized { key, inline }
            }
            _ => return Err(self.bad("unsupported parameter storage")),
        };
        Ok(Parameter {
            source: self.span(begin),
            argument,
            item,
            item_index,
            storage,
            flag,
            value,
        })
    }
    fn element(&mut self) -> Result<Element> {
        let begin = self.at;
        // 5468A0: version <= 3; versions 0/1 (registry ordinal, no word_10) are
        // not present in any original map and stay unsupported.
        let version = self.word()?;
        if !(2..=3).contains(&version) {
            self.at -= 4;
            return Err(self.bad(format!("unsupported element version {version}; expected 2 or 3")));
        }
        let name = self.identifier()?;
        let opcode = crate::registrations::element(&name)
            .ok_or_else(|| self.bad(format!("unknown element {name}")))?;
        let word_10 = self.word()?;
        let expected = crate::registrations::schema(opcode).map_or(0, |(_, a)| a.len());
        // 544190 reads the count itself for opcodes 71, 4 and 94 (and 36 below
        // version 3); every other element takes the generic 858AD0 check.
        let upgraded = matches!(opcode, 4 | 71 | 94) || (opcode == 36 && version < 3);
        let count = self.count(9)?;
        if count != expected {
            return Err(self.bad(if upgraded {
                format!("544190 parameter count {count} for opcode {opcode} (schema {expected}) is not ported")
            } else {
                "Unexpected number of parameters. (5468A0 -> 858AD0 throws)".to_string()
            }));
        }
        let mut parameters = Vec::new();
        parameters
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            parameters.push(self.parameter()?);
        }
        if opcode == 36 && version < 3 {
            return Err(self.bad(
                "544190 turns a version-2 element 36 text into a localized trigger_%d_element_%d key                  registered in the global KKC_TextDataManager (49AEF0); not ported",
            ));
        }
        if matches!(opcode, 4 | 94)
            && parameters
                .first()
                .is_some_and(|p| crate::registrations::argument(&p.argument) == Some(42))
        {
            return Err(self.bad(
                "544190 converts an ma_cutscene parameter to ma_string (549000 B43B40) with a warning; not ported",
            ));
        }
        Ok(Element {
            source: self.span(begin),
            name,
            opcode,
            word_10,
            parameters,
        })
    }
    fn elements(&mut self) -> Result<Vec<Element>> {
        let count = self.count(14)?;
        let mut out = Vec::new();
        out.try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            out.push(self.element()?);
        }
        Ok(out)
    }
    fn trigger(&mut self) -> Result<Trigger> {
        let begin = self.at;
        self.version(4, "trigger")?;
        let id = self.word()?;
        let mode = self.word()?;
        let enabled = self.byte()?;
        let repeat = self.byte()?;
        let activation_bits = self.word()?;
        let name = self.text()?;
        let description = self.text()?;
        let word_2c = self.word()?;
        let events = self.elements()?;
        let conditions = self.elements()?;
        let actions = self.elements()?;
        Ok(Trigger {
            source: self.span(begin),
            id,
            mode,
            enabled,
            repeat,
            activation_bits,
            name,
            description,
            word_2c,
            events,
            conditions,
            actions,
        })
    }
}
/// 54D3D0 hash fold: a-z to A-Z, every other unit unchanged.
fn fold(name: &[u16]) -> Vec<u16> {
    name.iter()
        .map(|&u| if (0x61..=0x7a).contains(&u) { u - 0x20 } else { u })
        .collect()
}
impl SavedManager {
    /// The caller verifies the RMP chunk's source identity, supplies the native
    /// localization stream mode and chooses an input allocation budget. This
    /// budget is a decoding guard, not a gameplay RAM or object limit.
    pub fn decode(bytes: &[u8], localization: Localization, byte_budget: usize) -> Result<Self> {
        if bytes.len() > byte_budget {
            return Err(Error::Budget {
                needed: bytes.len(),
                budget: byte_budget,
            });
        }
        let mut r = Reader {
            bytes,
            at: 0,
            localization,
        };
        // 54D3D0 reads versions 0..5; the original maps use 4 and 5 only.
        let version = r.word()?;
        if !(4..=5).contains(&version) {
            r.at = 0;
            return Err(r.bad(format!("unsupported manager version {version}; expected 4 or 5")));
        }
        let next_id = r.word()?;
        let count = r.count(38)?;
        let mut triggers = Vec::new();
        triggers
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            triggers.push(r.trigger()?);
        }
        let count = r.count(6)?;
        let mut variables = Vec::new();
        variables
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            let name = r.text()?;
            let bits = r.word()?;
            // 54D3D0 creates no variable for an empty name.
            if name.is_empty() {
                continue;
            }
            // The native map (54CB80 head insertion, uppercase-folded hash) keeps
            // duplicates and finds the newest; that lookup is not ported.
            if variables.iter().any(|v: &Variable| fold(&v.name) == fold(&name)) {
                return Err(r.bad("duplicate trigger variable name"));
            }
            variables.push(Variable { name, bits });
        }
        let mut strings = Vec::new();
        if version == 4 {
            // 54D79F: one string and one word, read and discarded.
            r.text()?;
            r.word()?;
        } else {
            let count = r.count(2)?;
            strings
                .try_reserve_exact(count)
                .map_err(|_| Error::Allocation)?;
            for _ in 0..count {
                strings.push(r.text()?);
            }
        }
        if r.at != bytes.len() {
            return Err(r.bad("trailing trigger bytes"));
        }
        Ok(Self {
            next_id,
            triggers,
            variables,
            strings,
        })
    }
}

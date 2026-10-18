//! Original GActor::Load tail `4D971B..4D99C2` (after the `4D67F0` refresh):
//! the `+158` required-object check, MapString validation of the saved name
//! key `+DC` against the actor's expected name, and the numbered-name fix /
//! assignment. This is the native code behind the restoration guard
//! "Actor name collision/action-table validation is unavailable" (any actor
//! with a nonempty `+DC`) and "Actor numbered-name assignment requires
//! current owner state". Also the Kingdom numbered-name counter `4E4980`.
//! Oracle: research/phase8-saved-components/native-actor-names.json.
//!
//! Native pieces: expected name `4D8CA0` (ActorData `+10` when `+170` is 0,
//! else the numbered name `4D8AB0`: localized format `actor_name_numbered
//! [_st|_nd|_rd]|ds` through `4B9F40`, CString::Format `405D60` ->
//! `_vswprintf`), `wcscmp` `96CB83`, `_swscanf(text, "%d")` `96C99A`, the
//! MapString pair reset `49AD00(B43B40, B43B48)` (both empty), map-string
//! lookup `4BAA20(create 0)`, Kingdom `4E4980` + `4CF1A0` (`+170`).
use crate::organization_state::{Arg, LoadError, Message, Result};

/// The actor fields the tail reads and writes.
pub struct SavedName<'a> {
    /// `+158` saved required object GTID (0 = none).
    pub required_158: u32,
    /// `+DC` MapString key and `+E0` its text.
    pub key: &'a mut Vec<u16>,
    pub text: &'a mut Vec<u16>,
    /// `+170` number of a numbered name (0 = none).
    pub number: &'a mut u32,
}

/// Memory facts ("fact") and ordered callbacks of the tail.
pub trait NameWorld {
    /// Fact: actor `+14` GTID.
    fn gtid(&self) -> Result<u32>;
    /// Fact: ActorData `+8` IDS (messages).
    fn definition_name(&self) -> Result<String>;
    /// Fact: ActorData `+10`, the stored `GetLocalized("name")` text.
    fn display_name(&self) -> Result<Vec<u16>>;
    /// Fact: ActorData `+1B0` (numbered).
    fn numbered(&self) -> Result<bool>;
    /// Fact: the GTID table `[B364C0]` resolves `id` (generation match and a
    /// nonnull object).
    fn gtid_live(&self, id: u32) -> Result<bool>;
    /// `4B9F40([B36544]; key, B43B48)`: the localized text, or None when the
    /// default (empty) is returned.
    fn localize(&mut self, key: &str) -> Result<Option<Vec<u16>>>;
    /// `59E520(load context)`: map strings are read from the stream.
    fn strings_from_stream(&mut self) -> Result<bool>;
    /// `4BAA20([B36544]; key, create 0)` found an entry.
    fn map_string_exists(&mut self, key: &[u16]) -> Result<bool>;
    /// `5A1040` map warning (the load continues).
    fn warn(&mut self, message: Message) -> Result<()>;
    /// Owner virtual `+108` (Kingdom); None is the native null. Only the
    /// gate: the assignment below uses the owner word `+E8`.
    fn owner_kingdom(&mut self) -> Result<Option<u32>>;
    /// Fact: actor `+E8` owner word (the `this` of `4E4980`).
    fn owner_word_e8(&self) -> Result<u32>;
    /// `4E4980(actor)` on Kingdom `owner_e8`: the assigned number (stored to
    /// `+170` by `4CF1A0`); bind [`NumberedNameCounters::assign`].
    fn assign_number(&mut self, owner_e8: u32) -> Result<u32>;
}

const DEFAULT_KEY: &str = "actor_name_numbered|ds";

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// `405D60` -> `_vswprintf(format, number, display)` for the shipped
/// formats: `%d` then `%s` (in that argument order) and `%%`. Any other
/// conversion would read the arguments with another type: refused.
pub fn format_numbered(format: &[u16], number: i32, display: &[u16]) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    let mut next = 0;
    let mut i = 0;
    while i < format.len() {
        let c = format[i];
        if c != u16::from(b'%') {
            out.push(c);
            i += 1;
            continue;
        }
        match format.get(i + 1).copied() {
            Some(c) if c == u16::from(b'%') => out.push(c),
            Some(c) if c == u16::from(b'd') && next == 0 => {
                out.extend(utf16(&number.to_string()));
                next = 1;
            }
            Some(c) if c == u16::from(b's') && next == 1 => {
                out.extend_from_slice(display);
                next = 2;
            }
            _ => {
                return Err(LoadError::Unsupported(format!(
                    "numbered-name format {:?} outside the %d/%s argument order",
                    String::from_utf16_lossy(format)
                )));
            }
        }
        i += 2;
    }
    Ok(out)
}

/// `4D8AB0`: the numbered name of `number` (signed `idiv` remainders).
pub fn numbered_name(number: i32, display: &[u16], world: &mut impl NameWorld) -> Result<Vec<u16>> {
    let units = number % 10;
    let tens = number % 100;
    let key = if tens > 10 && tens <= 20 {
        DEFAULT_KEY
    } else {
        match units {
            1 => "actor_name_numbered_st|ds",
            2 => "actor_name_numbered_nd|ds",
            3 => "actor_name_numbered_rd|ds",
            u if u == 0 || u >= 4 => DEFAULT_KEY,
            // Negative remainders fall through the native switch: the
            // string stays empty.
            _ => return Ok(Vec::new()),
        }
    };
    let format = world.localize(key)?.unwrap_or_default();
    format_numbered(&format, number, display)
}

/// `4D8CA0`: ActorData `+10` for `+170 == 0`, else the numbered name.
pub fn expected_name(number: u32, world: &mut impl NameWorld) -> Result<Vec<u16>> {
    let display = world.display_name()?;
    if number == 0 {
        Ok(display)
    } else {
        numbered_name(number as i32, &display, world)
    }
}

/// `96C99A _swscanf(text, "%d")` in the original's static "C" tables (only
/// `setlocale(LC_NUMERIC, ..)` is ever called): skip wide whitespace (the
/// `[B358A4]` table below 0x100: 09..0D, 20, A0), one optional sign, then
/// ASCII digits (narrow "C" `isdigit` for chars below 0x100), accumulated
/// in 64 bits and stored as the low 32 bits. None when no digit was read
/// (the target is not written). A char >= 0x100 reaching the whitespace
/// test is classified by Windows (`GetStringTypeW`), not by AA.exe: refused.
pub fn scan_decimal(text: &[u16]) -> Result<Option<u32>> {
    let space = |c: u16| matches!(c, 0x09..=0x0d | 0x20 | 0xa0);
    let mut i = 0;
    loop {
        match text.get(i).copied() {
            None | Some(0xffff) => return Ok(None),
            Some(c) if c >= 0x100 => {
                return Err(LoadError::Unsupported(format!(
                    "swscanf whitespace test of U+{c:04X} is an OS classification (GetStringTypeW)"
                )));
            }
            Some(c) if space(c) => i += 1,
            Some(_) => break,
        }
    }
    let mut negative = false;
    if let Some(&c) = text.get(i)
        && (c == u16::from(b'-') || c == u16::from(b'+'))
    {
        negative = c == u16::from(b'-');
        i += 1;
    }
    let mut value: u64 = 0;
    let mut digits = 0;
    while let Some(&c) = text.get(i) {
        if !(0x30..=0x39).contains(&c) {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(u64::from(c - 0x30));
        digits += 1;
        i += 1;
    }
    if digits == 0 {
        return Ok(None);
    }
    if negative {
        value = value.wrapping_neg();
    }
    Ok(Some(value as u32))
}

/// The tail `4D971B..4D99C2`. `actor_version` is the saved actor version
/// (signed compare with 0xE). Earlier stores stay on error, as natively.
pub fn finish(name: SavedName<'_>, actor_version: u32, world: &mut impl NameWorld) -> Result<()> {
    let SavedName {
        required_158,
        key,
        text,
        number,
    } = name;
    if required_158 != 0 && !world.gtid_live(required_158)? {
        return Err(LoadError::Warning(Message::new(
            "actor %s required object ID %d missing - did it fail to load or is this a load order problem?",
            vec![Arg::Text(world.definition_name()?), Arg::Int(required_158 as i32)],
        )));
    }
    let gtid = world.gtid()? as i32;
    let text_of = |v: &[u16]| Arg::Text(String::from_utf16_lossy(v));
    if !key.is_empty() {
        let expected = expected_name(*number, world)?;
        if *text == expected {
            world.warn(Message::new(
                "actor %d (%s) MapString %s deleted - is unnecessary",
                vec![Arg::Int(gtid), Arg::Text(world.definition_name()?), text_of(key)],
            ))?;
            key.clear();
            text.clear();
        } else {
            let missing = !world.strings_from_stream()? && !world.map_string_exists(key)?;
            if missing {
                world.warn(Message::new(
                    "actor %d (%s) MapString %s missing - keeping '%s'",
                    vec![Arg::Int(gtid), Arg::Text(world.definition_name()?), text_of(key), text_of(text)],
                ))?;
            }
        }
    }
    if !world.numbered()? {
        return Ok(());
    }
    if world.owner_kingdom()?.is_none() {
        return Ok(());
    }
    if *number != 0 {
        return Ok(());
    }
    if (actor_version as i32) < 0xe && !key.is_empty() {
        if let Some(value) = scan_decimal(text)? {
            *number = value;
        }
        if *number != 0 {
            world.warn(Message::new(
                "actor %d (%s) fixing numbered name (%d)",
                vec![Arg::Int(gtid), Arg::Text(world.definition_name()?), Arg::Int(*number as i32)],
            ))?;
            key.clear();
            text.clear();
        }
    }
    if *number == 0 {
        let owner = world.owner_word_e8()?;
        if owner == 0 {
            // 4E4980 on a null Kingdom: 5C37E0 reads [0x35C + 4].
            return Err(LoadError::NullDereference(0x5c3803));
        }
        *number = world.assign_number(owner)?;
        if key.is_empty() {
            world.warn(Message::new(
                "actor %d (%s) assigning numbered name (%d)",
                vec![Arg::Int(gtid), Arg::Text(world.definition_name()?), Arg::Int(*number as i32)],
            ))?;
        }
    }
    Ok(())
}

/// Kingdom `+35C` numbered-name counters (`4E4980`): per ActorData, the next
/// number is the previous one plus 1, starting at 1. Not saved with the
/// Kingdom: rebuilt by the assignments of the load (4D8CF0) and of owner
/// changes (4D6F10). Saved `+170` numbers are not registered here, exactly
/// as natively.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NumberedNameCounters<D> {
    entries: Vec<(D, u32)>,
}
impl<D: PartialEq + Clone> NumberedNameCounters<D> {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
    /// `4E4980` for an actor of `definition`: the number `4CF1A0` stores.
    pub fn assign(&mut self, definition: &D) -> Result<u32> {
        if let Some(entry) = self.entries.iter_mut().find(|(d, _)| d == definition) {
            let next = entry.1.wrapping_add(1);
            if next == 0 {
                // Native inserts a second node with 1 (4E49E7); its later
                // lookup order depends on the hash table layout.
                return Err(LoadError::Unsupported("numbered-name counter wrapped past 2^32".into()));
            }
            entry.1 = next;
            return Ok(next);
        }
        self.entries.push((definition.clone(), 1));
        Ok(1)
    }
    /// Current count for a definition (0 when never assigned).
    pub fn count(&self, definition: &D) -> u32 {
        self.entries.iter().find(|(d, _)| d == definition).map_or(0, |e| e.1)
    }
}

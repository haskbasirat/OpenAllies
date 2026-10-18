//! MAPSTRINGS: the map string table of an original RMP (KKC_Data::LoadMapStrings
//! 47EA40) and the 49AD70 binding of localized trigger parameters.
//!
//! Native path (proven by research/phase8-triggers/native_mapstrings.py on all
//! 80 original maps and 70+ synthetic probes): 473050 data-file reader, 5BD2C0
//! load (5AA990 memory file, 856F90 character reader, 857190/856D20 line
//! reader, 5BD080 TGC_DataFileParser with the 5BCC80/5BEFD0 tables, 494000
//! DisplayString escapes), "Text" node (5B4410) loaded into a KKC_TextData
//! (475180, vtable[0] 4BA710) registered in the KKC_TextDataManager [B36544]
//! (4BA2A0, DataCheckingStrict), lookups 4B9D40 / 4B9F40 GetDisplayString.
//!
//! Only the oracle-proven subset is accepted. Inputs the original rejects with
//! a C++ exception fail with [`ParseError::Native`]; everything else outside
//! the proven subset fails with [`ParseError::Unsupported`] instead of a guess.
use crate::decode::{SavedManager, Value};

/// Native throw sites of the original parser for inputs it rejects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeError {
    /// 5B3F9C: a key repeats in the node (case-insensitive).
    DuplicateKey,
    /// 5B56B4: the Text node has no `name` field.
    MissingName,
    /// 410CAE: "Value for key %s is not type DisplayString (6)." (unquoted).
    NotDisplayString,
    /// 5BEF00: "Mismatched quotes around localized string value."
    MismatchedQuotes,
    /// 5BA57B: "Non-template node had a field with no value specified."
    NoValue,
    /// 5BE961: "Expected key name, found nothing."
    EmptyKey,
    /// 5BC8A8: "Language-specific subnode found after default language
    /// subnode, default must be last."
    DefaultNotLast,
    /// 5BB03C: "Node with language-specific subnodes did not match current
    /// language and had no default."
    NoLanguageMatch,
}

impl NativeError {
    /// Return address of the original 96BEF5 throw call.
    pub fn throw_site(self) -> u32 {
        match self {
            Self::DuplicateKey => 0x5b3f9c,
            Self::MissingName => 0x5b56b4,
            Self::NotDisplayString => 0x410cae,
            Self::MismatchedQuotes => 0x5bef00,
            Self::NoValue => 0x5ba57b,
            Self::EmptyKey => 0x5be961,
            Self::DefaultNotLast => 0x5bc8a8,
            Self::NoLanguageMatch => 0x5bb03c,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("MAPSTRINGS line {line}: the original parser throws {kind:?} (site {:#x})", kind.throw_site())]
    Native { line: usize, kind: NativeError },
    #[error("MAPSTRINGS line {line}: {message}; outside the oracle-proven subset")]
    Unsupported { line: usize, message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    #[error(
        "localized key {key:?} is not in MAPSTRINGS; original 4B9F40 then searches the global \
         KKC_TextDataManager tables, warns (404ED0/404EA0) and falls back to the \
         \"error_missing_string\" key, none of which is bound here"
    )]
    Missing { key: String },
    #[error("localized key with non-ASCII UTF-16 units: native case-insensitive lookup is locale dependent")]
    KeyNotAscii,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Key exactly as written (ASCII); lookups are case-insensitive.
    pub key: String,
    /// Stored text (entry+0x10 CStringW): every byte widened to one UTF-16
    /// unit. 856F90 reads a chunk without BOM through CRT isleadbyte (96CBD7,
    /// _pctype) and mbtowc (9710C6, LC_CTYPE handle); the game never leaves
    /// the C locale (its only setlocale calls are LC_NUMERIC "C"), where no
    /// byte is a lead byte and mbtowc widens, independent of the Windows code
    /// page (proven for 0x80..0xFF by the oracle with the image's locale).
    pub text: Vec<u16>,
}

/// The selected `[Text]` node of a MAPSTRINGS chunk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapStrings {
    /// Entries in source order (the native list order differs; lookups only
    /// depend on keys, which are unique).
    pub entries: Vec<Entry>,
}

/// Original line limit: lines of up to 2047 bytes (excluding the CR of a CRLF)
/// load; 2048 are truncated by the line reader (proven both ways).
const MAX_LINE: usize = 2047;

struct Line<'a> {
    number: usize,
    bytes: &'a [u8],
}

struct Section<'a> {
    header_line: usize,
    language: Option<&'a [u8]>,
    fields: Vec<Line<'a>>,
}

fn unsupported<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError::Unsupported {
        line,
        message: message.into(),
    })
}

fn native<T>(line: usize, kind: NativeError) -> Result<T, ParseError> {
    Err(ParseError::Native { line, kind })
}

fn trim(mut s: &[u8]) -> &[u8] {
    while let [b' ' | b'\t', rest @ ..] = s {
        s = rest;
    }
    while let [rest @ .., b' ' | b'\t'] = s {
        s = rest;
    }
    s
}

/// Header `[Name]` or `[Name language = L]`, optionally followed by `{`.
/// Returns (language, opens_scope).
fn header<'a>(line: &Line<'a>) -> Result<(Option<&'a [u8]>, bool), ParseError> {
    let s = line.bytes;
    let close = match s.iter().position(|&b| b == b']') {
        Some(i) => i,
        None => return unsupported(line.number, "header without ']'"),
    };
    let opens = match trim(&s[close + 1..]) {
        b"" => false,
        b"{" => true,
        _ => return unsupported(line.number, "text after the header"),
    };
    let inner = trim(&s[1..close]);
    let name_end = inner
        .iter()
        .position(|&b| b == b' ' || b == b'\t')
        .unwrap_or(inner.len());
    if !inner[..name_end].eq_ignore_ascii_case(b"Text") {
        return unsupported(line.number, "only [Text] nodes are proven");
    }
    let attrs = trim(&inner[name_end..]);
    if attrs.is_empty() {
        return Ok((None, opens));
    }
    let eq = match attrs.iter().position(|&b| b == b'=') {
        Some(i) => i,
        None => return unsupported(line.number, "header attribute without '='"),
    };
    if !trim(&attrs[..eq]).eq_ignore_ascii_case(b"language") {
        return unsupported(line.number, "header attribute other than language");
    }
    let language = trim(&attrs[eq + 1..]);
    if language.is_empty() || !language.iter().all(u8::is_ascii_alphabetic) {
        return unsupported(line.number, "language value is not a plain word");
    }
    Ok((Some(language), opens))
}

/// Split a field line into key and value with the proven comment rule:
/// `;` after the last `"` of the value starts a comment, and `;` between the
/// quotes of a two-quote value is text. Other `;` placements are unsupported.
fn field<'a>(line: &Line<'a>) -> Result<(&'a [u8], &'a [u8]), ParseError> {
    let s = line.bytes;
    let eq = match s.iter().position(|&b| b == b'=') {
        Some(i) => i,
        None if s.contains(&b';') => return unsupported(line.number, "';' in a field without '='"),
        None => return native(line.number, NativeError::NoValue),
    };
    let key = trim(&s[..eq]);
    if key.is_empty() {
        return native(line.number, NativeError::EmptyKey);
    }
    if !key
        .iter()
        .all(|&b| (0x20..0x7f).contains(&b) && !b"\";[]{}".contains(&b))
    {
        return unsupported(line.number, "key characters outside printable ASCII names");
    }
    let mut value = &s[eq + 1..];
    let last_quote = value.iter().rposition(|&b| b == b'"');
    let quotes = value.iter().filter(|&&b| b == b'"').count();
    let mut commented = false;
    for (i, &b) in value.iter().enumerate() {
        if b != b';' {
            continue;
        }
        match last_quote {
            Some(q) if i < q && quotes == 2 => {} // inside "..."
            Some(q) if i < q => return unsupported(line.number, "';' between several quote pairs"),
            _ => {
                value = &value[..i];
                commented = true;
                break;
            }
        }
    }
    let value = trim(value);
    if value.is_empty() {
        if commented {
            return unsupported(line.number, "value consisting only of a comment");
        }
        return native(line.number, NativeError::NoValue);
    }
    Ok((key, value))
}

/// DisplayString value: outer quotes required, inner quotes removed, one
/// leading '@' removed. Escapes ('&', '\\') are not ported.
fn display_string(line: &Line, value: &[u8]) -> Result<Vec<u16>, ParseError> {
    if value[0] != b'"' {
        return native(line.number, NativeError::NotDisplayString);
    }
    if value[value.len() - 1] != b'"' {
        return native(line.number, NativeError::MismatchedQuotes);
    }
    let inner = if value.len() >= 2 { &value[1..value.len() - 1] } else { &[][..] };
    let mut text: Vec<u8> = inner.iter().copied().filter(|&b| b != b'"').collect();
    if text.contains(&b'&') || text.contains(&b'\\') {
        return unsupported(line.number, "DisplayString escape processing (494000) is not ported");
    }
    if text.first() == Some(&b'@') {
        text.remove(0);
    }
    Ok(text.iter().map(|&b| u16::from(b)).collect())
}

impl MapStrings {
    /// Parse a decompressed MAPSTRINGS chunk. `language` is the LanguageIDS
    /// game variable (the oracle ran with "English").
    pub fn parse(chunk: &[u8], language: &str) -> Result<Self, ParseError> {
        let (body, utf8_mode) = match chunk.strip_prefix(b"\xef\xbb\xbf") {
            Some(rest) => (rest, true),
            None => (chunk, false),
        };
        let pieces: Vec<&[u8]> = body.split(|&b| b == b'\n').collect();
        let mut lines = Vec::new();
        for (index, &raw) in pieces.iter().enumerate() {
            let number = index + 1;
            // The CR of a CRLF terminator; a CR elsewhere stays in the line.
            let raw = match raw.strip_suffix(b"\r") {
                Some(r) if index + 1 < pieces.len() => r,
                _ => raw,
            };
            if raw.len() > MAX_LINE {
                return unsupported(number, "line longer than the proven 2047 bytes");
            }
            if raw.contains(&0) {
                return unsupported(number, "NUL byte");
            }
            if utf8_mode && !raw.is_ascii() {
                return unsupported(number, "UTF-8 mode (BOM) decodes through MultiByteToWideChar");
            }
            lines.push(Line { number, bytes: raw });
        }
        // Structure: header, '{' (same or next line), fields, '}' (or EOF).
        let mut sections: Vec<Section> = Vec::new();
        #[derive(PartialEq)]
        enum State {
            Top,
            AfterHeader,
            InNode,
        }
        let mut state = State::Top;
        for line in lines {
            let t = trim(line.bytes);
            // Control and non-ASCII bytes are proven only inside quoted values.
            let first_quote = t.iter().position(|&b| b == b'"').unwrap_or(t.len());
            let last_quote = t.iter().rposition(|&b| b == b'"').map_or(t.len(), |q| q + 1);
            if t[..first_quote]
                .iter()
                .chain(&t[last_quote.max(first_quote)..])
                .any(|&b| b < 0x20 && b != b'\t' || b == 0x7f || b >= 0x80)
            {
                return unsupported(line.number, "control or non-ASCII byte outside a quoted value");
            }
            if t.is_empty() || t[0] == b';' {
                continue;
            }
            let line = Line {
                number: line.number,
                bytes: t,
            };
            match state {
                State::Top => {
                    if t[0] != b'[' {
                        return unsupported(line.number, "content outside a [Text] node");
                    }
                    let (language, opens) = header(&line)?;
                    sections.push(Section {
                        header_line: line.number,
                        language,
                        fields: Vec::new(),
                    });
                    state = if opens { State::InNode } else { State::AfterHeader };
                }
                State::AfterHeader => {
                    if t != b"{" {
                        return unsupported(line.number, "header not followed by '{'");
                    }
                    state = State::InNode;
                }
                State::InNode => {
                    if t == b"}" {
                        state = State::Top;
                    } else if matches!(t[0], b'[' | b'{' | b'}') {
                        return unsupported(line.number, "nested node or scope");
                    } else {
                        sections.last_mut().unwrap().fields.push(line);
                    }
                }
            }
        }
        if state == State::AfterHeader {
            return unsupported(sections.last().unwrap().header_line, "header without a scope");
        }
        let selected = Self::select(&sections, language)?;
        let mut entries: Vec<Entry> = Vec::new();
        let mut keys: Vec<&[u8]> = Vec::new();
        let mut has_name = false;
        for (i, section) in sections.iter().enumerate() {
            for line in &section.fields {
                let (key, value) = match field(line) {
                    Ok(kv) => kv,
                    // A skipped language node is not proven to be checked.
                    Err(_) if i != selected => {
                        return unsupported(line.number, "malformed field in an unselected language node");
                    }
                    Err(e) => return Err(e),
                };
                if i != selected {
                    if key.eq_ignore_ascii_case(b"ids") || key.eq_ignore_ascii_case(b"name") {
                        continue;
                    }
                    if display_string(line, value).is_err() {
                        return unsupported(line.number, "unproven value in an unselected language node");
                    }
                    continue;
                }
                if keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
                    return native(line.number, NativeError::DuplicateKey);
                }
                keys.push(key);
                if key.eq_ignore_ascii_case(b"ids") {
                    continue;
                }
                if key.eq_ignore_ascii_case(b"name") {
                    if value.len() < 2 || value[0] != b'"' || value[value.len() - 1] != b'"' {
                        return unsupported(line.number, "unquoted name (native throws from 5B8F5C)");
                    }
                    has_name = true;
                    continue;
                }
                entries.push(Entry {
                    key: String::from_utf8(key.to_vec()).expect("ASCII key"),
                    text: display_string(line, value)?,
                });
            }
        }
        if !has_name {
            return native(sections[selected].header_line, NativeError::MissingName);
        }
        Ok(Self { entries })
    }

    /// Language node selection (5BB0xx/5BC8A8): the first node matching
    /// LanguageIDS, else the trailing Default node.
    fn select(sections: &[Section], language: &str) -> Result<usize, ParseError> {
        if sections.is_empty() {
            return unsupported(0, "no [Text] node");
        }
        if sections.iter().any(|s| s.language.is_none()) {
            if sections.len() == 1 {
                return Ok(0);
            }
            return unsupported(sections[1].header_line, "several [Text] nodes without language");
        }
        let is_default = |s: &Section| s.language.unwrap().eq_ignore_ascii_case(b"Default");
        if let Some(d) = sections.iter().position(is_default)
            && d + 1 < sections.len()
        {
            return native(sections[d + 1].header_line, NativeError::DefaultNotLast);
        }
        if let Some(i) = sections
            .iter()
            .position(|s| s.language.unwrap().eq_ignore_ascii_case(language.as_bytes()))
        {
            return Ok(i);
        }
        if is_default(sections.last().unwrap()) {
            return Ok(sections.len() - 1);
        }
        native(sections[0].header_line, NativeError::NoLanguageMatch)
    }

    /// 4B9D40 lookup (case-insensitive key).
    pub fn get(&self, key: &[u16]) -> Result<Option<&Entry>, LookupError> {
        if key.iter().any(|&u| u >= 0x80) {
            return Err(LookupError::KeyNotAscii);
        }
        Ok(self.entries.iter().find(|e| {
            e.key.len() == key.len()
                && e
                    .key
                    .bytes()
                    .zip(key)
                    .all(|(a, &b)| a.eq_ignore_ascii_case(&(b as u8)))
        }))
    }

    /// 49AD70 external mode: an empty key gives the static empty CStringW
    /// B43B48 (constructed by 9E7400 -> 405B10, never assigned); otherwise
    /// 4B9F40 GetDisplayString(key, B43B48).
    pub fn resolve(&self, key: &[u16]) -> Result<Vec<u16>, LookupError> {
        if key.is_empty() {
            return Ok(Vec::new());
        }
        match self.get(key)? {
            Some(entry) => Ok(entry.text.clone()),
            None => Err(LookupError::Missing {
                key: String::from_utf16_lossy(key),
            }),
        }
    }

    /// Fill `Value::Localized.inline` of every parameter decoded in external
    /// mode (the original RMP case). Returns the number of parameters bound.
    pub fn bind(&self, saved: &mut SavedManager) -> Result<usize, LookupError> {
        let mut bound = 0;
        for t in &mut saved.triggers {
            for e in t.events.iter_mut().chain(&mut t.conditions).chain(&mut t.actions) {
                for p in &mut e.parameters {
                    if let Value::Localized { key, inline: inline @ None } = &mut p.value {
                        *inline = Some(self.resolve(key)?);
                        bound += 1;
                    }
                }
            }
        }
        Ok(bound)
    }
}

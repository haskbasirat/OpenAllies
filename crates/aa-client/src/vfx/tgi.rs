//! Tolerant reader for TimeGate `.tgi` definition files as used by the
//! effect tables: `[Kind Template=X]` headers with or without `{}` bodies,
//! `;` / `//` / `/* */` comments, `key = value` statements (optionally with a
//! leading type word such as `float`), quoted strings and comma lists.

#[derive(Clone, Debug, Default)]
pub struct Block {
    /// First word of the header (`VFX`, `Template`, `Flags`, `list`, ...), lowercase.
    pub kind: String,
    /// Remaining positional header words (e.g. the template name).
    pub words: Vec<String>,
    /// `key=value` header attributes (`template`, `inherit`), keys lowercase.
    pub attrs: Vec<(String, String)>,
    /// Statements in source order, keys lowercase.
    pub values: Vec<(String, String)>,
    pub children: Vec<Block>,
}

impl Block {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    /// Last value written for `key` (later statements override earlier ones).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    pub fn child(&self, kind: &str) -> Option<&Block> {
        self.children.iter().find(|c| c.kind == kind)
    }
    /// Template parent named by `Template=` or `inherit=`.
    pub fn parent(&self) -> Option<&str> {
        self.attr("template").or_else(|| self.attr("inherit"))
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Header(String),
    Open,
    Close,
    Statement(String),
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut in_block_comment = false;
    for raw in text.lines() {
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;
        let mut stmt = String::new();
        let mut quoted = false;
        let flush = |stmt: &mut String, tokens: &mut Vec<Token>| {
            let s = stmt.trim();
            if !s.is_empty() {
                tokens.push(Token::Statement(s.to_owned()));
            }
            stmt.clear();
        };
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            if in_block_comment {
                if c == '*' && next == Some('/') {
                    in_block_comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if quoted {
                stmt.push(c);
                if c == '"' {
                    quoted = false;
                }
                i += 1;
                continue;
            }
            match c {
                '"' => {
                    quoted = true;
                    stmt.push(c);
                }
                '/' if next == Some('*') => {
                    in_block_comment = true;
                    i += 2;
                    continue;
                }
                '/' if next == Some('/') => break,
                ';' => break,
                '[' => {
                    flush(&mut stmt, &mut tokens);
                    let mut header = String::new();
                    i += 1;
                    while i < chars.len() && chars[i] != ']' {
                        header.push(chars[i]);
                        i += 1;
                    }
                    tokens.push(Token::Header(header.trim().to_owned()));
                }
                '{' => {
                    flush(&mut stmt, &mut tokens);
                    tokens.push(Token::Open);
                }
                '}' => {
                    flush(&mut stmt, &mut tokens);
                    tokens.push(Token::Close);
                }
                _ => stmt.push(c),
            }
            i += 1;
        }
        flush(&mut stmt, &mut tokens);
    }
    tokens
}

fn header_block(header: &str) -> Block {
    let mut block = Block::default();
    // `Template=X` may be written with spaces around '='.
    let normalized = header.replace(" =", "=").replace("= ", "=");
    for (n, word) in normalized.split_whitespace().enumerate() {
        if let Some((k, v)) = word.split_once('=') {
            block
                .attrs
                .push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        } else if n == 0 {
            block.kind = word.to_ascii_lowercase();
        } else {
            block.words.push(word.to_owned());
        }
    }
    block
}

/// Split one statement line into `key = value` pairs. A line may hold several
/// pairs (`IDS = a  model = b`); type words before a key are ignored.
fn statements(line: &str, out: &mut Vec<(String, String)>) {
    // lex: words, quoted strings, '='
    let mut lex: Vec<(String, bool)> = Vec::new(); // (text, is_eq)
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '=' {
            lex.push(("=".into(), true));
            i += 1;
        } else if c == '"' {
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            i += 1;
            lex.push((s, false));
        } else {
            let mut s = String::new();
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' {
                s.push(chars[i]);
                i += 1;
            }
            lex.push((s, false));
        }
    }
    let mut n = 0;
    let mut pending_words: Vec<String> = Vec::new();
    while n < lex.len() {
        if lex[n].1 {
            // key is the word right before '='
            let key = pending_words.pop().unwrap_or_default().to_ascii_lowercase();
            pending_words.clear();
            n += 1;
            let mut value: Vec<String> = Vec::new();
            while n < lex.len() {
                // stop before the next `key =`
                if !lex[n].1 && n + 1 < lex.len() && lex[n + 1].1 && !value.is_empty() {
                    let prev_comma = value.last().is_some_and(|v| v.ends_with(','));
                    if !prev_comma {
                        break;
                    }
                }
                if lex[n].1 {
                    break;
                }
                value.push(lex[n].0.clone());
                n += 1;
            }
            if !key.is_empty() {
                out.push((key, value.join(" ")));
            }
        } else {
            pending_words.push(lex[n].0.clone());
            n += 1;
        }
    }
    // A bare `flag name` declaration (templates): record as present.
    if !lex.iter().any(|(_, eq)| *eq)
        && let Some(last) = pending_words.last()
    {
        out.push((last.to_ascii_lowercase(), String::new()));
    }
}

/// Parse a whole file into its top-level blocks.
pub fn parse(text: &str) -> Vec<Block> {
    let tokens = tokenize(text);
    let mut pos = 0;
    let mut root = Block::default();
    body(&tokens, &mut pos, &mut root, true, true);
    root.children
}

fn body(tokens: &[Token], pos: &mut usize, block: &mut Block, braced: bool, top: bool) {
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Statement(s) => {
                statements(s, &mut block.values);
                *pos += 1;
            }
            Token::Close => {
                if braced && !top {
                    *pos += 1;
                }
                if top {
                    // stray close at top level: skip it
                    *pos += 1;
                    continue;
                }
                return;
            }
            Token::Open => {
                // anonymous braces merge into the current block
                *pos += 1;
                body(tokens, pos, block, true, false);
            }
            Token::Header(h) => {
                if !braced {
                    return; // a brace-less body ends at the next header
                }
                let mut child = header_block(h);
                *pos += 1;
                if matches!(tokens.get(*pos), Some(Token::Open)) {
                    *pos += 1;
                    body(tokens, pos, &mut child, true, false);
                } else {
                    body(tokens, pos, &mut child, false, false);
                }
                block.children.push(child);
            }
        }
    }
}

/// Floats of a value like `.7,.7,.4`, `(1, 0, 0)` or `1 1 1`.
pub fn floats(value: &str) -> Vec<f32> {
    value
        .split(|c: char| c == ',' || c.is_whitespace() || c == '(' || c == ')')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f32>().ok())
        .collect()
}

pub fn boolean(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braced_and_braceless_blocks() {
        let text = r##"
/* header
   [Not A Block] */
[VFX Template=AAVFXModelTemporary]
{
	IDS = plane_death   ; comment
	name = "#burning effects"
	model = BurningEffects/AirplaneDeath.nif

	[Flags]

	random_orientation = true
}
[VFX Template=VFXTracer] { IDS = rifle_shot_tracer color=.7,.7,.4 speed=100 length = 2 }
[template VFXScorchMark inherit=VFXProjectedTexture]
{
	float texture_size = 4
	flag random_orientation = true
	point2 decal_fade_time = 0, 5
}
"##;
        let blocks = parse(text);
        assert_eq!(blocks.len(), 3);
        let a = &blocks[0];
        assert_eq!(a.kind, "vfx");
        assert_eq!(a.parent(), Some("AAVFXModelTemporary"));
        assert_eq!(a.get("ids"), Some("plane_death"));
        assert_eq!(a.get("name"), Some("#burning effects"));
        assert_eq!(
            a.child("flags").and_then(|f| f.get("random_orientation")),
            Some("true")
        );
        let b = &blocks[1];
        assert_eq!(b.get("ids"), Some("rifle_shot_tracer"));
        assert_eq!(floats(b.get("color").unwrap()), vec![0.7, 0.7, 0.4]);
        assert_eq!(b.get("speed"), Some("100"));
        assert_eq!(b.get("length"), Some("2"));
        let c = &blocks[2];
        assert_eq!(c.kind, "template");
        assert_eq!(c.words, vec!["VFXScorchMark".to_string()]);
        assert_eq!(c.get("texture_size"), Some("4"));
        assert_eq!(floats(c.get("decal_fade_time").unwrap()), vec![0.0, 5.0]);
    }
}

//! Tokenized ordered TGI syntax. Source bytes are retained, not reinterpreted as
//! UTF-8 or evaluated. Braced nodes and the native unbraced child-node form are
//! distinct. Comments/strings are lexed before structural delimiters.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenKind {
    Atom,
    Quoted,
    OpenHeader,
    CloseHeader,
    OpenBody,
    CloseBody,
    Equals,
    Newline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub span: Span,
    pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub span: Span,
    /// Type/name/declaration tokens, in their original order and spelling.
    pub left: Vec<Token>,
    /// None means a declaration such as `string IDS`; Some(empty span) keeps
    /// an explicitly empty assignment distinct from an absent assignment.
    pub value: Option<Span>,
    pub value_tokens: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub span: Span,
    pub header: Header,
    pub braced: bool,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    Node(Node),
    Property(Property),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    source: Vec<u8>,
    pub items: Vec<Item>,
    /// Native line/EOF tolerances used explicitly by `parse_native`; never edits source bytes.
    pub notices: Vec<SyntaxNotice>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxNoticeKind {
    HeaderEndedAtLineEnd,
    BodyEndedAtEof,
    /// An unbraced node's properties are followed by `{`: the native parser
    /// (5BD080 handler table 5BCC80) opens that body for the same node, and
    /// its `}` returns to the parent (oracle: native-tgi-parser.json).
    DelayedBody,
    /// A `"` after the first value token is an ordinary value character in
    /// 5BDF80; it never opens a quoted span.
    ValueQuoteAsText,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxNotice {
    pub kind: SyntaxNoticeKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_tokens: 4_000_000,
            max_depth: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("TGI syntax at byte {offset}: {message}")]
pub struct ParseError {
    pub offset: usize,
    pub message: &'static str,
}
fn error(offset: usize, message: &'static str) -> ParseError {
    ParseError { offset, message }
}

impl Document {
    pub fn parse(source: impl Into<Vec<u8>>) -> Result<Self, ParseError> {
        Self::parse_with_limits(source, Limits::default())
    }
    pub fn parse_with_limits(
        source: impl Into<Vec<u8>>,
        limits: Limits,
    ) -> Result<Self, ParseError> {
        Self::parse_mode(source.into(), limits, false)
    }
    /// Bounded native grammar: a header ends at its physical line boundary and
    /// plain open bodies finalize at EOF. Other unsupported grammar still errors.
    pub fn parse_native(source: impl Into<Vec<u8>>, limits: Limits) -> Result<Self, ParseError> {
        Self::parse_mode(source.into(), limits, true)
    }
    fn parse_mode(source: Vec<u8>, limits: Limits, native: bool) -> Result<Self, ParseError> {
        if source.len() > limits.max_bytes {
            return Err(error(0, "input byte limit exceeded"));
        }
        let (tokens, lexical_notices) = lex(&source, limits.max_tokens, native)?;
        let mut parser = Parser {
            source: &source,
            tokens: &tokens,
            at: 0,
            eof: source.len(),
            max_depth: limits.max_depth,
            native,
            notices: lexical_notices,
        };
        let items = parser.items(false, false, 0)?;
        if parser.at != tokens.len() {
            return Err(error(tokens[parser.at].span.start, "unconsumed token"));
        }
        let notices = parser.notices;
        Ok(Self {
            source,
            items,
            notices,
        })
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.source
    }
    pub fn bytes(&self, span: Span) -> Option<&[u8]> {
        self.source.get(span.start..span.end)
    }
    pub fn text(&self, span: Span) -> Option<&str> {
        std::str::from_utf8(self.bytes(span)?).ok()
    }
    pub fn header_name<'a>(&'a self, header: &Header) -> Option<&'a str> {
        self.text(header.tokens.first()?.span)
    }
    pub fn property_name<'a>(&'a self, property: &Property) -> Option<&'a str> {
        self.text(property.left.last()?.span)
    }
    /// Raw expression text only; no quotes removed, type coercion, reference
    /// resolution, expression evaluation or inheritance is performed.
    pub fn property_value<'a>(&'a self, property: &Property) -> Option<&'a str> {
        self.text(property.value?).map(str::trim)
    }
    pub fn nodes(&self) -> Vec<&Node> {
        fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a Node>) {
            for item in items {
                if let Item::Node(node) = item {
                    out.push(node);
                    walk(&node.items, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.items, &mut out);
        out
    }
}

fn lex(
    source: &[u8],
    limit: usize,
    native: bool,
) -> Result<(Vec<Token>, Vec<SyntaxNotice>), ParseError> {
    let mut tokens = Vec::new();
    let mut in_header = false;
    let mut seen_equals = false;
    // Native 5BDF80: only a value whose first character is `"` is quoted; the
    // quote ends at the next `"` on the same line, without escapes.
    let mut value_token_seen = false;
    let mut notices = Vec::new();
    let mut at = if source.starts_with(&[0xef, 0xbb, 0xbf]) {
        3
    } else {
        0
    };
    while at < source.len() {
        let start = at;
        let ch = source[at];
        if ch == b' ' || ch == b'\t' || ch == 0x0b || ch == 0x0c {
            at += 1;
            continue;
        }
        if ch == b';' || (ch == b'/' && source.get(at + 1) == Some(&b'/')) {
            while at < source.len() && !matches!(source[at], b'\r' | b'\n') {
                at += 1;
            }
            continue;
        }
        if ch == b'/' && source.get(at + 1) == Some(&b'*') {
            at += 2;
            while at + 1 < source.len() && &source[at..at + 2] != b"*/" {
                at += 1;
            }
            if at + 1 >= source.len() {
                return Err(error(start, "unterminated block comment"));
            }
            at += 2;
            continue;
        }
        let kind = match ch {
            b'\r' | b'\n' => {
                seen_equals = false;
                value_token_seen = false;
                if native {
                    in_header = false;
                }
                at += 1;
                if ch == b'\r' && source.get(at) == Some(&b'\n') {
                    at += 1;
                }
                TokenKind::Newline
            }
            b'[' => {
                in_header = true;
                at += 1;
                TokenKind::OpenHeader
            }
            b']' => {
                in_header = false;
                at += 1;
                TokenKind::CloseHeader
            }
            b'{' => {
                seen_equals = false;
                at += 1;
                TokenKind::OpenBody
            }
            b'}' => {
                seen_equals = false;
                at += 1;
                TokenKind::CloseBody
            }
            b'=' => {
                seen_equals = true;
                value_token_seen = false;
                at += 1;
                TokenKind::Equals
            }
            b'"' if native && seen_equals && !value_token_seen => {
                at += 1;
                while at < source.len() && !matches!(source[at], b'"' | b'\r' | b'\n') {
                    at += 1;
                }
                if source.get(at) != Some(&b'"') {
                    // 5BEE60 throws "Mismatched quotes around localized string value."
                    return Err(error(start, "unterminated quoted string"));
                }
                at += 1;
                TokenKind::Quoted
            }
            b'"' if !(native && seen_equals) => {
                at += 1;
                let mut closed = false;
                while at < source.len() {
                    if source[at] == b'\\' {
                        at += 1;
                        if at < source.len() {
                            at += 1;
                        }
                        continue;
                    }
                    if source[at] == b'"' {
                        at += 1;
                        closed = true;
                        break;
                    }
                    at += 1;
                }
                if !closed {
                    return Err(error(start, "unterminated quoted string"));
                }
                TokenKind::Quoted
            }
            0 => return Err(error(at, "NUL byte is not TGI text")),
            _ => {
                let value_text = native && seen_equals;
                if ch == b'"' {
                    notices.push(SyntaxNotice {
                        kind: SyntaxNoticeKind::ValueQuoteAsText,
                        span: Span {
                            start,
                            end: start + 1,
                        },
                    });
                }
                at += 1;
                // A slash inside an atom belongs to that atom (e.g. http:// or
                // an asset wildcard). Comments start between tokens.
                while at < source.len()
                    // Original key scanner 0x5BE8A0 terminates at '=', ';',
                    // '/' or line end, NOT square brackets. Keep brackets
                    // inside property-name atoms (e.g. shipped key_[).
                    && (!(in_header || seen_equals) || !matches!(source[at], b'[' | b']'))
                    && !matches!(
                        source[at],
                        b' ' | b'\t'
                            | b'\r'
                            | b'\n'
                            | b'{'
                            | b'}'
                            | b'='
                            | b';'
                            | 0
                    )
                    && (value_text || source[at] != b'"')
                {
                    at += 1;
                }
                TokenKind::Atom
            }
        };
        if tokens.len() >= limit {
            return Err(error(start, "token limit exceeded"));
        }
        if seen_equals && matches!(kind, TokenKind::Atom | TokenKind::Quoted) {
            value_token_seen = true;
        }
        tokens.push(Token {
            kind,
            span: Span { start, end: at },
        });
    }
    Ok((tokens, notices))
}

struct Parser<'a> {
    source: &'a [u8],
    tokens: &'a [Token],
    at: usize,
    eof: usize,
    max_depth: usize,
    native: bool,
    notices: Vec<SyntaxNotice>,
}
impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }
    fn skip_newlines(&mut self) {
        while self.peek().is_some_and(|t| t.kind == TokenKind::Newline) {
            self.at += 1;
        }
    }
    fn items(
        &mut self,
        in_body: bool,
        until_header: bool,
        depth: usize,
    ) -> Result<Vec<Item>, ParseError> {
        if depth > self.max_depth {
            return Err(error(
                self.peek().map_or(self.eof, |t| t.span.start),
                "nesting limit exceeded",
            ));
        }
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            let Some(token) = self.peek() else {
                return Ok(out);
            };
            match token.kind {
                TokenKind::CloseBody if in_body => return Ok(out),
                TokenKind::OpenHeader if until_header => return Ok(out),
                TokenKind::OpenHeader => out.push(Item::Node(self.node(depth + 1)?)),
                TokenKind::CloseBody => {
                    return Err(error(token.span.start, "unexpected closing brace"));
                }
                TokenKind::OpenBody => {
                    return Err(error(token.span.start, "body has no preceding node header"));
                }
                TokenKind::CloseHeader => {
                    return Err(error(token.span.start, "unexpected closing bracket"));
                }
                _ => out.push(Item::Property(self.property()?)),
            }
        }
    }
    fn node(&mut self, depth: usize) -> Result<Node, ParseError> {
        let start = self.peek().unwrap().span.start;
        self.at += 1;
        let mut header_tokens = Vec::new();
        let header_end = loop {
            let token = match self.peek() {
                Some(token) => token,
                None if self.native => {
                    self.notices.push(SyntaxNotice {
                        kind: SyntaxNoticeKind::HeaderEndedAtLineEnd,
                        span: Span {
                            start,
                            end: self.eof,
                        },
                    });
                    break self.eof;
                }
                None => return Err(error(start, "unterminated node header")),
            };
            self.at += 1;
            match token.kind {
                TokenKind::CloseHeader => break token.span.end,
                TokenKind::OpenHeader | TokenKind::OpenBody | TokenKind::CloseBody => {
                    return Err(error(
                        token.span.start,
                        "invalid structural token in header",
                    ));
                }
                TokenKind::Newline if self.native => {
                    self.notices.push(SyntaxNotice {
                        kind: SyntaxNoticeKind::HeaderEndedAtLineEnd,
                        span: Span {
                            start,
                            end: token.span.start,
                        },
                    });
                    break token.span.start;
                }
                TokenKind::Newline => {}
                _ => header_tokens.push(token),
            }
        };
        if header_tokens.is_empty() {
            return Err(error(start, "empty node header"));
        }
        let header = Header {
            span: Span {
                start,
                end: header_end,
            },
            tokens: header_tokens,
        };
        self.skip_newlines();
        let mut braced = self.peek().is_some_and(|t| t.kind == TokenKind::OpenBody);
        if braced {
            self.at += 1;
        }
        let mut items = if !braced && self.native {
            self.unbraced_native(depth)?
        } else {
            self.items(true, !braced, depth)?
        };
        if !braced && self.peek().is_some_and(|t| t.kind == TokenKind::OpenBody) {
            // Only reached in native mode: the unbraced item run stopped at `{`.
            self.at += 1;
            self.notices.push(SyntaxNotice {
                kind: SyntaxNoticeKind::DelayedBody,
                span: Span {
                    start,
                    end: header_end,
                },
            });
            items.extend(self.items(true, false, depth)?);
            braced = true;
        }
        let end = if braced {
            if let Some(token) = self.peek() {
                if token.kind != TokenKind::CloseBody {
                    return Err(error(token.span.start, "expected closing brace"));
                }
                self.at += 1;
                token.span.end
            } else if self.native {
                self.notices.push(SyntaxNotice {
                    kind: SyntaxNoticeKind::BodyEndedAtEof,
                    span: Span {
                        start,
                        end: self.eof,
                    },
                });
                self.eof
            } else {
                return Err(error(start, "unterminated node body"));
            }
        } else {
            items.last().map_or(header_end, |item| match item {
                Item::Node(n) => n.span.end,
                Item::Property(p) => p.span.end,
            })
        };
        Ok(Node {
            span: Span { start, end },
            header,
            braced,
            items,
        })
    }
    /// Native unbraced node items: properties until the next header, the
    /// parent's `}`, EOF, or a `{` that opens this node's delayed body.
    fn unbraced_native(&mut self, depth: usize) -> Result<Vec<Item>, ParseError> {
        if depth > self.max_depth {
            return Err(error(
                self.peek().map_or(self.eof, |t| t.span.start),
                "nesting limit exceeded",
            ));
        }
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            let Some(token) = self.peek() else {
                return Ok(out);
            };
            match token.kind {
                TokenKind::OpenHeader | TokenKind::CloseBody | TokenKind::OpenBody => {
                    return Ok(out);
                }
                TokenKind::CloseHeader => {
                    return Err(error(token.span.start, "unexpected closing bracket"));
                }
                _ => out.push(Item::Property(self.property()?)),
            }
        }
    }
    /// Parse-time value checks of the native handlers (oracle cases in
    /// research/phase8-restoration/native-tgi-parser.json): a value that
    /// starts with `"` must end with `"` (5BEE60), a `string` declaration
    /// rejects quotes, a `localized` declaration requires quotes or null.
    fn native_value_checks(
        &self,
        left: &[Token],
        values: &[Token],
        value: Option<Span>,
    ) -> Result<(), ParseError> {
        let (Some(value), Some(first)) = (value, values.first()) else {
            return Ok(());
        };
        let raw = &self.source[value.start..value.end];
        let quoted = first.kind == TokenKind::Quoted;
        if quoted && raw.last() != Some(&b'"') {
            return Err(error(
                first.span.start,
                "mismatched quotes around localized string value",
            ));
        }
        if left.len() == 2 {
            let kind = &self.source[left[0].span.start..left[0].span.end];
            if kind.eq_ignore_ascii_case(b"string") && quoted {
                return Err(error(
                    first.span.start,
                    "quotes are reserved for localized strings",
                ));
            }
            if kind.eq_ignore_ascii_case(b"localized") && !quoted && !raw.eq_ignore_ascii_case(b"null")
            {
                return Err(error(
                    first.span.start,
                    "illegal value for localized type (missing quotes)",
                ));
            }
        }
        Ok(())
    }
    fn property(&mut self) -> Result<Property, ParseError> {
        let start = self.peek().unwrap().span.start;
        let mut left = Vec::new();
        let mut values = Vec::new();
        let mut equals = None;
        while let Some(token) = self.peek() {
            if matches!(
                token.kind,
                TokenKind::Newline | TokenKind::OpenHeader | TokenKind::CloseBody
            ) {
                break;
            }
            if matches!(token.kind, TokenKind::OpenBody | TokenKind::CloseHeader) {
                return Err(error(
                    token.span.start,
                    "unexpected structural token in property",
                ));
            }
            self.at += 1;
            if token.kind == TokenKind::Equals && equals.is_none() {
                equals = Some(token);
            } else if equals.is_some() {
                values.push(token);
            } else {
                left.push(token);
            }
        }
        if left.is_empty() {
            return Err(error(start, "property has no name/declaration"));
        }
        let end = values
            .last()
            .or(equals.as_ref())
            .or(left.last())
            .unwrap()
            .span
            .end;
        let value = equals.map(|operator| Span {
            start: values.first().map_or(operator.span.end, |t| t.span.start),
            end,
        });
        if self.native {
            self.native_value_checks(&left, &values, value)?;
        }
        Ok(Property {
            span: Span { start, end },
            left,
            value,
            value_tokens: values,
        })
    }
}

/// Native stored text of a quoted (localized) value: 5BEE60 requires the
/// first and last characters to be `"` (or the value `null`, kept verbatim),
/// 5BDC00 removes every `"`, and 5BA3C0 drops one leading `@`. The result is
/// the string-table key or literal text; it is not resolved here.
pub fn native_localized_text(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.first() == Some(&b'"') && bytes.last() == Some(&b'"') {
        let text: String = raw.chars().filter(|c| *c != '"').collect();
        Some(text.strip_prefix('@').map_or(text.clone(), str::to_owned))
    } else if raw.eq_ignore_ascii_case("null") {
        Some(raw.to_owned())
    } else {
        None
    }
}

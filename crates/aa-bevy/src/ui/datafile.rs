//! A small reader for the original game-data TGI files the front end and
//! HUD need (Data/Game/resources.tgi, actor_commands*.tgi, Nations,
//! RandomMap ...): nodes `[Name template=X]` with braced or unbraced
//! bodies, fields `[type] key = value`, `;` line and `/* */` block
//! comments. aa-ui's TGI store only resolves the UI templates, so these
//! files are read structurally here (values as written, no inheritance).
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    /// Bare header words after the name (`[Template Foo ...]` -> ["Foo"]).
    pub args: Vec<String>,
    /// `key=value` attributes of the header (`template`, `inherit`...).
    pub attrs: BTreeMap<String, String>,
    pub fields: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
    pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.fields.iter().filter(move |(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
    pub fn template(&self) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k.eq_ignore_ascii_case("template")).map(|(_, v)| v.as_str())
    }
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name.eq_ignore_ascii_case(name))
    }
    pub fn float(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|v| v.trim().trim_end_matches(['f', 'F']).parse().ok())
    }
    pub fn int(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(|v| v.trim().parse().ok())
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.get(key).map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_str = false;
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '"' {
                in_str = false;
            }
            if c == '\n' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = ' ';
                for d in chars.by_ref() {
                    if prev == '*' && d == '/' {
                        break;
                    }
                    if d == '\n' {
                        out.push('\n');
                    }
                    prev = d;
                }
            }
            ';' | '#' => {
                for d in chars.by_ref() {
                    if d == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(v).to_string()
}

fn header(line: &str) -> Option<Node> {
    let inner = line.trim().strip_prefix('[')?.split(']').next()?;
    let mut parts = inner.split_whitespace();
    let name = parts.next()?.to_string();
    let rest: Vec<&str> = parts.collect();
    let rest = rest.join(" ");
    let mut attrs = BTreeMap::new();
    // "template=X", "Template = X", "Inherit=Y"
    let norm = rest.replace(" = ", "=").replace("= ", "=").replace(" =", "=");
    let mut args = Vec::new();
    for tok in norm.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            attrs.insert(k.to_ascii_lowercase(), v.to_string());
        } else {
            args.push(tok.to_string());
        }
    }
    Some(Node { name, args, attrs, ..Default::default() })
}

/// Parse a file into its top-level nodes.
pub fn parse(text: &str) -> Vec<Node> {
    let text = strip_comments(text);
    // a stack of open nodes; `braced` tells whether a `{` opened it
    let mut root = Node::default();
    let mut stack: Vec<(Node, bool)> = Vec::new();
    let mut pending: Option<Node> = None;
    fn close(stack: &mut Vec<(Node, bool)>, root: &mut Node) {
        if let Some((n, _)) = stack.pop() {
            match stack.last_mut() {
                Some((p, _)) => p.children.push(n),
                None => root.children.push(n),
            }
        }
    }
    for raw in text.lines() {
        let mut line = raw.trim();
        while !line.is_empty() {
            if line.starts_with('[') {
                // an unbraced node ends at the next header of its level
                if let Some(p) = pending.take() {
                    stack.push((p, false));
                }
                while stack.last().is_some_and(|(_, braced)| !braced) {
                    close(&mut stack, &mut root);
                }
                pending = header(line);
                line = line.split_once(']').map(|(_, r)| r.trim()).unwrap_or("");
                continue;
            }
            if let Some(rest) = line.strip_prefix('{') {
                let n = pending.take().unwrap_or_default();
                stack.push((n, true));
                line = rest.trim();
                continue;
            }
            if let Some(rest) = line.strip_prefix('}') {
                if let Some(p) = pending.take() {
                    stack.push((p, false));
                }
                while stack.last().is_some_and(|(_, braced)| !braced) {
                    close(&mut stack, &mut root);
                }
                close(&mut stack, &mut root);
                line = rest.trim();
                continue;
            }
            // a field: [type] key = value
            if let Some(p) = pending.take() {
                stack.push((p, false));
            }
            let (lhs, value) = match line.split_once('=') {
                Some((l, v)) => (l.trim(), unquote(v)),
                None => (line, String::new()),
            };
            let key = lhs.split_whitespace().last().unwrap_or("").to_string();
            if !key.is_empty() {
                match stack.last_mut() {
                    Some((n, _)) => n.fields.push((key, value)),
                    None => root.fields.push((key, value)),
                }
            }
            line = "";
        }
    }
    if let Some(p) = pending.take() {
        stack.push((p, false));
    }
    while !stack.is_empty() {
        close(&mut stack, &mut root);
    }
    root.children
}

/// Read and parse a Data-relative file through the UI data root.
pub fn load(data: &aa_ui::fs::DataRoot, path: &str) -> Vec<Node> {
    match data.read(path) {
        Ok(bytes) => parse(&aa_ui::data::decode(&bytes)),
        Err(_) => Vec::new(),
    }
}

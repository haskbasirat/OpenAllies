/// Restricted native405D60 string formatting: one %s argument or a literal.
/// Other conversion directives are rejected rather than interpreted differently.
fn expand(format: &str, prefix: &str) -> Result<String> {
    let mut out = String::new();
    let mut chars = format.chars();
    let mut used = false;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('s') if !used => {
                out.push_str(prefix);
                used = true;
            }
            _ => anyhow::bail!("Unsupported original model format expression"),
        }
    }
    Ok(out)
}
fn model_path(folder: &str, resource: &str, aspect: &str, format: &str) -> Result<String> {
    let prefix = if aspect.starts_with('/') {
        aspect.to_owned()
    } else {
        format!("{resource}{aspect}")
    };
    let expanded = expand(format, &prefix)?;
    ensure!(!expanded.is_empty(), "Source aspect has no model");
    let path = if expanded.starts_with('/') {
        expanded.trim_start_matches('/').to_owned()
    } else if folder.is_empty() {
        expanded
    } else {
        format!("{folder}/{expanded}")
    };
    //85A030 resolves parent components before resource lookup. This portable
    //domain accepts regular relative paths only; drive/device paths stay errors.
    ensure!(
        !path.contains(':') && !path.contains('\0'),
        "Unsupported model resource path"
    );
    let normalized = path.replace('\\', "/");
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                ensure!(parts.pop().is_some(), "Model path escapes resource root");
            }
            _ => parts.push(part),
        }
    }
    ensure!(!parts.is_empty(), "Empty model resource path");
    Ok(parts.join("/"))
}

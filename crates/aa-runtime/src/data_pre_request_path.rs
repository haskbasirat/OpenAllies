//!Unchanged restricted405D60/4763C0 path domain formerly in client live_spawn.
//!Formatting a source filename does not acquire or initialize a resource.
type Result<T> = std::result::Result<T, String>;
///4763C0's source-file-relative operation. This does not apply85A030 parent
///normalization, printf expansion or a filesystem lookup. Empty remains empty.
pub fn relative(folder: &str, value: &str) -> Result<String> {
    if folder.contains('\0') || value.contains('\0') {
        return Err(
            "4763C0 embedded-NUL counted source path is outside the admitted domain".into(),
        );
    }
    Ok(if value.is_empty() {
        String::new()
    } else if value.starts_with('/') {
        value.trim_start_matches('/').to_owned()
    } else if folder.is_empty() {
        value.to_owned()
    } else {
        format!("{folder}/{value}")
    })
}
///One%s conversion or a literal. Other native variadic formats remain outside
///this source-bound portable domain instead of receiving invented arguments.
pub fn expand(format: &str, prefix: &str) -> Result<String> {
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
                used = true
            }
            _ => return Err("Unsupported original model format expression".into()),
        }
    }
    Ok(out)
}
pub fn model_path(folder: &str, resource: &str, aspect: &str, format: &str) -> Result<String> {
    let prefix = if aspect.starts_with('/') {
        aspect.to_owned()
    } else {
        format!("{resource}{aspect}")
    };
    let expanded = expand(format, &prefix)?;
    if expanded.is_empty() {
        return Err("Source aspect has no model".into());
    }
    let path = if expanded.starts_with('/') {
        expanded.trim_start_matches('/').to_owned()
    } else if folder.is_empty() {
        expanded
    } else {
        format!("{folder}/{expanded}")
    };
    //85A030 resolves parent components before lookup. Drive/device syntax is
    //outside this admitted relative source domain, exactly as the prior helper.
    if path.contains(':') || path.contains('\0') {
        return Err("Unsupported model resource path".into());
    }
    let normalized = path.replace('\\', "/");
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("Model path escapes resource root".into());
                }
            }
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err("Empty model resource path".into());
    }
    Ok(parts.join("/"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_source_model_path_fixture_uses_shared_lower_helper() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase7-live-spawn/native-model-binding.json"
        ))
        .unwrap();
        for row in fixture["paths"].as_array().unwrap() {
            assert_eq!(
                model_path(
                    row["folder"].as_str().unwrap(),
                    row["resource_prefix"].as_str().unwrap(),
                    row["aspect_prefix"].as_str().unwrap(),
                    row["model"].as_str().unwrap()
                )
                .unwrap(),
                row["result"].as_str().unwrap()
            );
        }
        for expression in ["%d.nif", "%s%s.nif", "%", "/../../bad.nif"] {
            assert!(model_path("Units/USA", "base", "", expression).is_err())
        }
        assert_eq!(expand("%s%%.nif", "name").unwrap(), "name%.nif");
    }
    #[test]
    fn original_v14_relative_source_path_preserves_verbatim_parent_and_format_text() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-data-pre-request/native-relative-source-path.json"
        ))
        .unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(), 63);
        for row in fixture["rows"].as_array().unwrap() {
            assert_eq!(
                relative(
                    row["folder"].as_str().unwrap(),
                    row["value"].as_str().unwrap()
                )
                .unwrap(),
                row["output"].as_str().unwrap()
            );
        }
        assert!(relative("folder", "a\0b").is_err());
    }
}

//! Parse every UI TGI file with the native-tolerant grammar and report.
use aa_ui::data::parse_source;
use aa_ui::fs::DataRoot;

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out)
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tgi")) {
            out.push(p)
        }
    }
}

fn main() {
    let root = DataRoot::workspace_default();
    let mut files = Vec::new();
    walk(&root.root().join("UI"), &mut files);
    walk(&root.root().join("Templates/UI"), &mut files);
    walk(&root.root().join("Fonts"), &mut files);
    let (mut ok, mut bad) = (0, 0);
    for f in files {
        let rel = f.strip_prefix(root.root()).unwrap().to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        match parse_source(&format!("/{rel}"), std::fs::read(&f).unwrap()) {
            Ok(s) => {
                ok += 1;
                if !s.notices.is_empty() {
                    println!("notice {rel}: {:?}", s.notices);
                }
                if !s.stray_fields.is_empty() {
                    println!("stray {rel}: {:?}", s.stray_fields.iter().map(|f| &f.left).collect::<Vec<_>>());
                }
            }
            Err(e) => {
                bad += 1;
                println!("ERR {e}");
            }
        }
    }
    println!("ok {ok} bad {bad}");
}

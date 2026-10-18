//! Load the UI lists and print a resolved node: `dump_node <file:path>...`
use aa_ui::fs::DataRoot;
use aa_ui::tgi::TgiStore;

fn main() {
    let mut store = TgiStore::new(DataRoot::workspace_default());
    store.load_ui_templates();
    println!("templates: {}", store.template_count());
    for spec in std::env::args().skip(1) {
        match store.node(&spec) {
            Ok(n) => println!("{n}"),
            Err(e) => println!("ERR {e}"),
        }
    }
    println!("diagnostics: {}", store.diagnostics.len());
    for d in store.diagnostics.iter().take(60) {
        println!("  {d}");
    }
}

use aa_formats::RwdArchive;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let command = args
        .next()
        .ok_or("usage: rwd list ARCHIVE | rwd extract-one ARCHIVE MEMBER OUTPUT_DIR")?;
    let archive = PathBuf::from(args.next().ok_or("missing archive path")?);
    let mut rwd = RwdArchive::open(&archive)?;
    if command == "list" {
        if args.next().is_some() {
            return Err("list takes only an archive path".into());
        }
        println!("source_index\tsize\tabsolute_offset\tpath");
        for entry in rwd.entries() {
            println!(
                "{}\t{}\t{}\t{}",
                entry.source_index(),
                entry.size(),
                entry.offset(),
                entry.path()
            );
        }
    } else if command == "extract-one" {
        let member = args
            .next()
            .ok_or("missing member name")?
            .into_string()
            .map_err(|_| "member name must be Unicode")?;
        let output = PathBuf::from(args.next().ok_or("missing output directory")?);
        if args.next().is_some() {
            return Err("unexpected argument".into());
        }
        let index = rwd.find(&member)?.ok_or("member not found")?;
        let path = rwd.extract_one(index, output)?;
        println!("{}", path.display());
    } else {
        return Err("unknown command; use list or extract-one".into());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

# aa-formats

Read-only **uncompressed TGCK v2 RWD** support for the independent Rust rebuild.
This is an archive foundation, not a recreated gameplay engine.

```rust
use aa_formats::RwdArchive;
let mut archive = RwdArchive::open("Data.rwd")?;
let index = archive.find("AVars_version.tgi")?.expect("member");
archive.copy_entry(index, &mut std::io::sink())?;
# Ok::<(), aa_formats::ArchiveError>(())
```

```text
cargo run -p aa-formats --bin rwd -- list "../Data.rwd"
cargo run -p aa-formats --bin rwd -- extract-one "../Data.rwd" "AVars_version.tgi" "output"
cargo test -p aa-formats
```

`entries()` retains footer order, exact original UTF-16 names, offsets, size,
flags and source index. `chunks()` also retains the opaque u64 and fixed UTF-16
name field. Lookup normalizes slash direction and ASCII case only; member
spelling is retained. Payload copies are streaming and byte-exact, with no
archive-size, member-count, bandwidth or whole-payload allocation cap. Integer
bounds come from actual file/chunk extents; metadata grows as it is parsed.
Compressed chunks and unknown member flags fail explicitly.

Extraction rejects traversal, rooted/drive/ADS paths, Windows device names,
ambiguous ASCII-case paths, existing output files and existing symlink/reparse
ancestors. Use an output directory controlled exclusively by the caller:
portable standard-library checks cannot prevent hostile concurrent ancestor
renames. A failed copy can leave its newly created partial file, and returns an
error; the crate never overwrites or silently reports it as complete. The source
archive must remain stable while open. No payload checksum is invented because
the recovered metadata does not establish one.

The format is grounded in `reconstruction/tools/archive_extract.py` and the
original installed ten RWDs. Tests include a 732-byte exact original member
with provenance, malformed metadata/UTF-16/path cases, sink/source failures,
safe output handling and a sparse 5 GiB logical fixture that reads under 1 KiB.
The optional ignored installed-archive test reads only metadata and that small
member when `AA_GAME_ROOT` is set. It never modifies the installation.

Each archive reader has its own seek position. Threads can open independent
readers, or share immutable metadata at a higher layer. This crate creates no
background threads and imposes no worker-count policy.

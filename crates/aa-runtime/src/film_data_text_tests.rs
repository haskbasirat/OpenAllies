use super::*;
use serde_json::Value;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn native_memory_writer_filename_forwarding_and_release_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film-data-text/native-data-text.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        let native = &row["native"];
        let name: Vec<u16> = input["name"].as_str().unwrap().encode_utf16().collect();
        assert_eq!(
            String::from_utf16(&sidecar_name(&name)).unwrap(),
            native["filename"].as_str().unwrap()
        );
        // This test binds only the private buffer kernel. It creates no work-
        // path receipt, Application, native lookup result or film metadata.
        let mut stream = PreparedDataText {
            path: None,
            bytes: Some(Vec::with_capacity(0x4000)),
        };
        let logical: Vec<u8> = input["text"]
            .as_str()
            .unwrap()
            .chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap())
            .collect();
        stream.build_current(&logical).unwrap();
        assert_eq!(hex(stream.bytes().unwrap()), native["built"]["bytes"]);
        assert_eq!(hex(stream.bytes().unwrap()), native["forwarded"]);
        assert_eq!(hex(stream.bytes().unwrap()), native["separate"]);
        assert_eq!(
            stream.bytes().unwrap().len() as u64,
            native["built"]["length"]
        );
        assert_eq!(native["initial"]["flags"], 0x6010a);
        assert_eq!(native["initial"]["capacity"], 0x4000);
        assert_eq!(
            native["initial"]["encoding_bytes"],
            serde_json::json!([0, 0, 0])
        );
        if logical.len() >= 0x4000 {
            assert!(native["built"]["capacity"].as_u64().unwrap() > 0x4000);
        }
        stream.release().unwrap();
        assert!(stream.bytes().is_err());
        assert!(stream.build_current(b"after release").is_err());
        assert!(stream.release().is_err());
        assert_eq!(native["released"]["buffer"], 0);
        assert_eq!(native["released"]["length"], 0);
        count += 1;
    }
    assert_eq!(count, 42);
}

#[test]
fn logical_input_refusal_keeps_the_same_prior_buffer() {
    let mut stream = PreparedDataText {
        path: None,
        bytes: Some(Vec::new()),
    };
    stream.build_current(b"prior\n").unwrap();
    assert!(stream.build_current(b"unbound\0tail").is_err());
    assert_eq!(stream.bytes().unwrap(), b"prior\r\n");
    stream.build_current(b"\r\n").unwrap();
    assert_eq!(stream.bytes().unwrap(), b"prior\r\n\r\r\n");
}

#[test]
fn native_separate_file_open_write_flush_payload_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film-data-text/native-file-text.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let logical: Vec<u8> = row["input"]["text"]
            .as_str()
            .unwrap()
            .chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap())
            .collect();
        let mut stream = PreparedDataText {
            path: None,
            bytes: Some(Vec::with_capacity(0x4000)),
        };
        stream.build_current(&logical).unwrap();
        assert_eq!(hex(stream.bytes().unwrap()), row["native"]["bytes"]);
        let open = &row["native"]["calls"][1];
        assert_eq!(open[0], "CreateFileW");
        assert_eq!(open[2], 0x4000_0000u32); // GENERIC_WRITE
        assert_eq!(open[3], 0); // native no-share
        assert_eq!(open[4], 2); // CREATE_ALWAYS
        assert_eq!(open[5], 0x80); // FILE_ATTRIBUTE_NORMAL
        count += 1;
    }
    assert_eq!(count, 12);
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT source completion and rewrite Userdata file IO"]
fn actual_constructed_fields_use_one_application_film_and_work_file() {
    use crate::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, film_create::Depot,
        load_view_variables::NativeOverrides,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap());
    catalog.native_load_mode = Some(1);
    let mut process = crate::film_process_metadata::State::from_fresh_sources(
        &catalog,
        &mut sources,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let variables = crate::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut sources,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let creator =
        crate::resource_creator::Creator::from_load_constructor(&catalog, &variables).unwrap();
    let header = {
        let bank = catalog.definition_bank.borrow();
        crate::film_header_owner::State::from_film_constructor(
            &catalog,
            &bank,
            &mut process,
            || Ok([0x12345678, 0x9abcdef0]),
        )
        .unwrap()
    };
    // Exact source-bound field constructors, NOT a5A31D0 managed-world capture.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let ids: Vec<u16> = format!("DataText-{}-{nonce}.RFM", std::process::id())
        .encode_utf16()
        .collect();
    let mut film = Resource::from_constructed_fields(ids.clone(), header, creator);
    let mut app = Application::from_sources(sources);
    let output_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-film-data-text/backend-output");
    std::fs::create_dir_all(&output_root).unwrap();
    app.configure_portable_work_depot(&output_root).unwrap();
    {
        let mut depot = app
            .film_depot(&catalog, NativeOverrides::ControlledNone)
            .unwrap();
        assert!(depot.find(&ids).unwrap().is_none());
        depot.ensure_work_depot(&mut film).unwrap();
        assert!(depot.work_is_topmost(&film).unwrap());
        depot.publish(&mut film).unwrap();
    }
    app.open_film_file(&mut film)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut prepared = PreparedDataText::prepare(&mut app, &film).unwrap();
    let path = prepared.filename().unwrap().path().to_path_buf();
    let film_path = app.work_path(&ids).unwrap();
    assert_eq!(path.extension().unwrap(), "TGI");
    assert!(!path.try_exists().unwrap());
    let mut logical = b"[Text language=Default]\n{\n\tname = \"Current\"\n}\n".to_vec();
    prepared.build_current(&logical).unwrap();
    logical.fill(b'X'); // Current stream owns its native build prefix, no stale source loan.
    prepared.ensure_existing_work(&mut app).unwrap();
    let expected = prepared.bytes().unwrap().to_vec();
    film.open_chunk("MAPSTRINGS", 0).unwrap();
    prepared.serialize_into(&mut film).unwrap();
    // External-file insertion AFTER verified absent lookup is a labelled IO
    // race probe; it proves delete is real rather than a missing-file no-op.
    std::fs::write(&path, b"external stale file").unwrap();
    prepared.remove_separate_file().unwrap();
    assert!(!path.try_exists().unwrap());
    prepared.remove_separate_file().unwrap(); // Native FALSE ignored too.
    prepared.save_separate().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    let closed = film.close().unwrap();
    assert_eq!(std::fs::read(&film_path).unwrap(), closed);
    let mut reader =
        aa_metagame::chunk_file::ChunkFileReader::open("actual-datatext", &closed).unwrap();
    assert_eq!(reader.chunk("MAPSTRINGS").unwrap(), expected);
    // A genuine now-existing sidecar must not be reclassified as absent.
    assert!(PreparedDataText::prepare(&mut app, &film)
        .err()
        .unwrap()
        .contains("DataFileResource"));
    prepared.remove_separate_file().unwrap();
    drop(app);
    assert!(prepared.remove_separate_file().is_err());
    assert_eq!(prepared.bytes().unwrap(), expected);
    prepared.release().unwrap();
    assert!(prepared.save_separate().is_err());
    // Only this test's two resolved regular files are removed; no recursive IO.
    std::fs::remove_file(&film_path).unwrap();
    println!("actual constructors, shared work depot, raw MAPSTRINGS, real TGI write/delete and dead-owner guard passed");
}

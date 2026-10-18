use aa_metagame::chunk_file::{ChunkFileReader,ChunkFileWriter};
use std::io::Read;

fn hex(bytes:&[u8])->String {bytes.iter().map(|b|format!("{b:02x}")).collect()}
#[test]
fn compressed_chunks_keep_stream_across_flush_and_native_directory_layout() {
    let mut random=Vec::new();let mut seed=0x12345678u32;
    for _ in 0..8193 {seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;random.push(seed as u8);}
    let cases=vec![vec![("EMPTY",true,vec![])],
        vec![("TEXT",true,"Original Axis & Allies — 日本語".repeat(200).into_bytes())],
        vec![("FIRST",true,random),("RAW",false,vec![17,18,19]),("TAIL",true,vec![0;12000])]];
    let mut reports=Vec::new();
    for (case,chunks) in cases.into_iter().enumerate() {
        let mut writer=ChunkFileWriter::create("interop.rfm");
        for (name,compressed,payload) in &chunks {
            writer.open_chunk(name,*compressed).unwrap();
            for piece in payload.chunks(97) {
                writer.write(piece).unwrap();
                writer.flush().unwrap();
            }
            assert_eq!(writer.current_logical_size().unwrap(),payload.len() as u64);
        }
        let bytes=writer.close().unwrap();
        let reader=ChunkFileReader::open("interop.rfm",&bytes).unwrap();
        assert_eq!(reader.chunks.len(),chunks.len());
        let mut end=12u64;let mut rows=Vec::new();
        for (record,(name,compressed,payload)) in reader.chunks.iter().zip(&chunks) {
            assert_eq!(record.c_name(),*name);
            assert_eq!(record.offset,end);
            assert_eq!(record.compressed(),*compressed);
            assert_eq!(record.uncompressed_size,payload.len() as u64);
            let stored=&bytes[record.offset as usize..(record.offset+record.size) as usize];
            let mut decoded=Vec::new();
            if *compressed {flate2::read::ZlibDecoder::new(stored).read_to_end(&mut decoded).unwrap();}
            else {decoded.extend_from_slice(stored);}
            assert_eq!(&decoded,payload);
            end+=record.size;
            rows.push(serde_json::json!({"name":name,"compressed":compressed,"payload":hex(payload),
                "stored":hex(stored),"offset":record.offset,"size":record.size,
                "logical_size":record.uncompressed_size}));
        }
        assert_eq!(bytes.len() as u64,end+reader.chunks.len() as u64*0x60);
        reports.push(serde_json::json!({"case":case,"file":hex(&bytes),"chunks":rows}));
    }
    if let Some(path)=std::env::var_os("AA_COMPRESSED_CHUNKS_EXPORT") {
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&serde_json::json!({"schema":1,"cases":reports,
            "qualification":"zlib representation compatibility; not original deflater byte parity"})).unwrap();
    }
}

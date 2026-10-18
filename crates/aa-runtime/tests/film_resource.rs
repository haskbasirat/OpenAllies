use aa_metagame::chunk_file::ChunkFileReader;
use aa_runtime::film_resource::Resource;

#[test]
fn actual_single_stream_attachment_and_lifetime_no_copy_or_aba() {
    let mut resource = Resource::open_memory("Films/test.RFM");
    assert!(resource.recorder_handle().is_err());
    resource.open_chunk("SessionPlayers", 0).unwrap();
    resource.write(&0u32.to_le_bytes()).unwrap();
    resource.flush().unwrap();
    assert!(resource.recorder_handle().is_err());
    resource.open_chunk("Orders", 0).unwrap();
    assert!(resource.recorder_handle().is_err());
    resource.flush().unwrap();
    let recorder = resource.recorder_handle().unwrap();
    let second = recorder.clone();
    recorder.write(&3u32.to_le_bytes()).unwrap();
    resource.write(&[1, 2]).unwrap();
    second.write(&[3]).unwrap();
    // Native same stream stays valid through another flush, without epoch caps.
    resource.flush().unwrap();
    recorder.write(&[4]).unwrap();
    let bytes = resource.close().unwrap();
    assert!(!recorder.is_live());
    assert!(second.write(&[5]).is_err());
    assert!(resource.close().is_err());
    let mut file = ChunkFileReader::open("Films/test.RFM", &bytes).unwrap();
    assert_eq!(file.chunk("SessionPlayers").unwrap(), &0u32.to_le_bytes());
    assert_eq!(file.chunk("Orders").unwrap(), &[3, 0, 0, 0, 1, 2, 3, 4]);
    drop(resource);
    let mut later = Resource::open_memory("Films/test.RFM");
    later.open_chunk("Orders", 0).unwrap();
    later.flush().unwrap();
    assert!(later.recorder_handle().unwrap().is_live());
    assert!(!recorder.is_live());
    drop(later);
    assert!(second.write(&[9]).is_err());
}

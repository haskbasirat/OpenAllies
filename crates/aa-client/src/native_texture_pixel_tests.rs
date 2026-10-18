use super::*;
fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|v|u8::from_str_radix(std::str::from_utf8(v).unwrap(),16).unwrap()).collect()}
#[test]
fn defined_native_reader_pixels_match_actual_decoder(){
    let data:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-texture-reader/native-reader-pixels.json")).unwrap();
    let mut count=0;
    for row in data["rows"].as_array().unwrap(){
        let bytes=hex(row["input"]["bytes"].as_str().unwrap());
        let encoding=match row["input"]["extension"].as_str().unwrap(){"dds"=>Encoding::Dds,"tga"=>Encoding::Tga,"png"=>Encoding::Png,_=>panic!("fixture reader")};
        //The old grayscale PNG reader has allocator-dependent pixel reads.
        //Its metadata is separately proved; no native gray-pixel parity claim.
        if encoding==Encoding::Png && matches!(bytes[25],0|4){continue}
        let decoded=decode_bytes(&bytes,encoding).unwrap_or_else(|error|panic!("{}: {error}",row["input"]["label"]));
        assert_eq!(decoded.levels()[0].pixels(),hex(row["output"]["normalized_rgba"].as_str().unwrap()),"{} / {}",row["precision"],row["input"]["label"]);
        count+=1;
    }
    assert_eq!(count,90);
}

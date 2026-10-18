use super::*;
fn dds(width:u32,height:u32,mips:u32,fourcc:&[u8;4],data:&[u8])->Vec<u8>{
 let mut bytes=vec![0;128];bytes[..4].copy_from_slice(b"DDS ");
 for(o,v)in[(4,124u32),(8,0x1007),(12,height),(16,width),(28,mips),(76,32),(80,4),(108,if mips>1{0x401008}else{0x1000})]{bytes[o..o+4].copy_from_slice(&v.to_le_bytes())}
 bytes[84..88].copy_from_slice(fourcc);bytes.extend_from_slice(data);bytes
}
#[test]fn dxt1_alpha_and_actual_sub_block_mip_levels_survive(){
 let mut payload=Vec::new();payload.extend_from_slice(&[0,0,255,255,255,255,255,255]);
 payload.extend_from_slice(&[0,248,31,0,0,0,0,0]);payload.extend_from_slice(&[224,7,0,0,0,0,0,0]);
 let bytes=dds(4,4,3,b"DXT1",&payload);let pixels=decode_bytes(&bytes,Encoding::Dds).unwrap();
 assert_eq!(pixels.levels().iter().map(|m|(m.width(),m.height())).collect::<Vec<_>>(),[(4,4),(2,2),(1,1)]);
 assert!(pixels.levels()[0].pixels().chunks_exact(4).all(|p|p==[0,0,0,0]));
 assert!(pixels.levels()[1].pixels().chunks_exact(4).all(|p|p==[255,0,0,255]));
 assert_eq!(pixels.levels()[2].pixels(),[0,255,0,255]);
}
#[test]fn dxt3_explicit_alpha_and_dxt5_interpolated_alpha_survive(){
 let mut d3=vec![0x10,0x32,0x54,0x76,0x98,0xba,0xdc,0xfe];d3.extend_from_slice(&[0,248,31,0,0,0,0,0]);
 let pixels=decode_bytes(&dds(4,4,1,b"DXT3",&d3),Encoding::Dds).unwrap();
 assert_eq!(pixels.levels()[0].pixels().chunks_exact(4).map(|p|p[3]).collect::<Vec<_>>(),(0u8..16).map(|n|n*17).collect::<Vec<_>>());
 let mut d5=vec![128,64,2,0,0,0,0,0];d5.extend_from_slice(&[0,248,31,0,0,0,0,0]);
 let pixels=decode_bytes(&dds(4,4,1,b"DXT5",&d5),Encoding::Dds).unwrap();assert_eq!(pixels.levels()[0].pixels()[3],118);
}
#[test]fn rgba_dds_masks_and_native_ignored_header_pitch_preserve_channels(){
 let mut bytes=dds(1,2,1,b"NONE",&[3,2,1,4,9,9,9,9,7,6,5,8,9,9,9,9]);
 for(o,v)in[(8,0x100fu32),(20,8),(80,0x41),(88,32),(92,0xff0000),(96,0xff00),(100,0xff),(104,0xff000000)]{bytes[o..o+4].copy_from_slice(&v.to_le_bytes())}
 bytes[84..88].fill(0);let pixels=decode_bytes(&bytes,Encoding::Dds).unwrap();assert_eq!(pixels.levels()[0].pixels(),[1,2,3,4,9,9,9,9]);
 assert!(decode_bytes(&bytes[..135],Encoding::Dds).is_err());
}
#[test]fn tga_origin_and_rle_preserve_original_alpha(){
 let mut bytes=vec![0;18];bytes[2]=2;bytes[12]=2;bytes[14]=2;bytes[16]=32;bytes[17]=8;
 bytes.extend_from_slice(&[255,0,0,32,255,255,255,64,0,0,255,255,0,255,0,128]);
 let pixels=decode_bytes(&bytes,Encoding::Tga).unwrap();assert_eq!(pixels.levels()[0].pixels(),[255,0,0,255,0,255,0,128,0,0,255,32,255,255,255,64]);
 let mut rle=vec![0;18];rle[2]=10;rle[12]=1;rle[14]=2;rle[16]=32;rle[17]=8;rle.extend_from_slice(&[0x81,255,0,0,128]);
 assert_eq!(decode_bytes(&rle,Encoding::Tga).unwrap().levels()[0].pixels(),[0,0,255,128,0,0,255,128]);
}
#[test]fn truncated_and_multidimensional_dds_never_substitute_an_image(){
 let mut bytes=dds(4,4,1,b"DXT1",&[0;8]);assert!(decode_bytes(&bytes[..135],Encoding::Dds).is_err());
 bytes[112..116].copy_from_slice(&0x200u32.to_le_bytes());assert!(decode_bytes(&bytes,Encoding::Dds).is_err());
}
#[test]#[ignore="requires original AA_GAME_ROOT source depot"]
fn actual_selected_texture_bytes_and_resource_identity_survive_cpu_decode(){
 use aa_runtime::{catalog::Catalog,combat_source_receipt::VerifiedStartupReceipt,resource_depot::Application};
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
 catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 let selected=files.catalog().into_iter().find(|f|f.logical_path.to_ascii_lowercase().ends_with("skirttexture.dds")).unwrap().logical_path;
 let expected=aa_rules::sources::sha256(&files.read(&selected).unwrap().bytes);
 let bank=std::mem::take(&mut*catalog.definition_bank.borrow_mut());let mut app=Application::from_sources(files);let mut resources=app.resources();
 let handle=resources.texture_5a77e0(&catalog,&bank,&selected.encode_utf16().collect::<Vec<_>>(),0).unwrap().unwrap();
 let before=(resources.flags_08(&handle).unwrap(),resources.references_14(&handle).unwrap(),resources.requests_04(&handle).unwrap());
 let input=resources.texture_input_4c(&handle).unwrap();assert_eq!(input.sha256(),expected);
 let decoded=decode(input).map_err(|f|f.error).unwrap();assert!(decoded.handle().same_identity(&handle));assert_eq!(decoded.source_sha256(),expected);
 assert!(!decoded.pixels().levels().is_empty());
 assert_eq!(before,(resources.flags_08(&handle).unwrap(),resources.references_14(&handle).unwrap(),resources.requests_04(&handle).unwrap()));
 let input=resources.texture_input_4c(&handle).unwrap();
 let failure=match input.decode_with(|_,_|Err("explicit decoder refusal".into())){Ok(_)=>panic!("refusal became success"),Err(f)=>f};
 assert!(failure.input.handle().same_identity(&handle));assert_eq!(failure.input.sha256(),expected);
}

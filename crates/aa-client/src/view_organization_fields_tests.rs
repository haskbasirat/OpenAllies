use super::*;
use serde_json::{Value,json};

#[test]
fn native_44fdb0_known_masks_allocation_and_retained_prefix(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/view-organization-fields/native-organization-fields.json")).unwrap();
    let mut rows=0;
    for row in doc["rows"].as_array().unwrap(){
        let cutoff=row["input"]["cutoff"].as_bool().unwrap();
        let result=if cutoff{Fields::construct_with(|_|anyhow::bail!("allocator-entry-cutoff"))}else{Fields::construct()};
        let(fields,error)=match result{Ok(fields)=>(fields,None),Err(f)=>(f.fields,Some(f.error.to_string()))};
        assert_eq!(error.as_deref().unwrap_or("ok"),row["output"]["outcome"].as_str().unwrap());
        for(offset,expected)in row["output"]["words"].as_object().unwrap(){
            let offset:usize=offset.parse().unwrap();
            //Native writes one byte16C, so its full scalar word stays unknown.
            let expected=if offset==0x16c{Value::Null}else{expected.clone()};
            assert_eq!(json!(fields.word(offset).unwrap()),expected,"row{rows} offset{offset:X}");
        }
        assert_eq!(json!(fields.byte_16c()),row["output"]["byte_16c"]);
        assert_eq!(fields.array.as_ref().unwrap().allocated,row["output"]["allocated"].as_bool().unwrap());
        assert_eq!(fields.require_constructed().is_ok(),!cutoff);
        assert_eq!(fields.word(0x15c).unwrap(),Some(0));assert_eq!(fields.word(0x160).unwrap(),Some(14));
        if cutoff{
            assert_eq!(fields.word(0x158).unwrap(),Some(0));
            assert_eq!(fields.word(0x164).unwrap(),None);assert_eq!(fields.word(0x168).unwrap(),None);
            assert_eq!(fields.byte_16c(),None);assert_eq!(fields.word(0x170).unwrap(),None);
        }else{
            assert!(fields.array.as_ref().unwrap().items.capacity()>=14);
            assert!(fields.array.as_ref().unwrap().items.is_empty());
            assert_eq!(row["output"]["known_masks"]["364"],json!(255));
        }
        rows+=1;
    }
    assert_eq!(rows,24);
}

#[test]
fn constructed_tail_exposes_null_links_but_no_padding_or_numeric_pointer(){
    let fields=Fields::construct().unwrap();fields.require_constructed().unwrap();
    assert_eq!(fields.word(0x158).unwrap(),None);
    for offset in[0x15c,0x164,0x168,0x170]{assert_eq!(fields.word(offset).unwrap(),Some(0))}
    assert_eq!(fields.byte_16c(),Some(0));assert_eq!(fields.word(0x16c).unwrap(),None);
    assert!(fields.word(0x154).is_err());assert!(fields.word(0x174).is_err());assert!(fields.word(0x169).is_err());
}

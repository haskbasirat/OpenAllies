type Result<T>=std::result::Result<T,String>;
pub(crate) fn subfolder_candidate(ids: &[u16], subfolder: &[u16]) -> Result<Vec<u16>> {
    if ids.contains(&0) || subfolder.contains(&0) || ids.iter().chain(subfolder).any(|&v| v > 127) {
        return Err("native Resource counted path is outside proved plain ASCII domain".into());
    }
    let split = ids
        .iter()
        .rposition(|&v| v == u16::from(b'/') || v == u16::from(b'\\'));
    let (dir, base) = if let Some(i) = split {
        (&ids[..i], &ids[i + 1..])
    } else if let Some(i) = ids.iter().rposition(|&v| v == u16::from(b':')) {
        (&ids[..=i], &ids[i + 1..])
    } else {
        (&[][..], ids)
    };
    fn join(a: &[u16], b: &[u16]) -> Vec<u16> {
        let mut out = a.to_vec();
        if b.first().is_some_and(|&v| v == 47 || v == 92) {
            if out.last().is_some_and(|&v| v == 47 || v == 92) {
                out.pop();
            }
        } else if out.last().is_some_and(|&v| v != 47 && v != 92) {
            out.push(47);
        }
        out.extend_from_slice(b);
        out
    }
    Ok(join(&join(dir, subfolder), base))
}
pub(crate) fn model_variant(
    ids: &[u16],
    hardware: bool,
    suffix: Option<&[u16]>,
) -> Result<Vec<u16>> {
    if ids.contains(&0) || ids.iter().any(|&v| v > 127) {
        return Err("native Model path outside proved plain ASCII counted domain".into());
    }
    if !hardware {
        return Ok(ids.to_vec());
    }
    let prefix = ids
        .iter()
        .rposition(|&v| v == 46)
        .map_or(ids, |i| &ids[..i]);
    let base = ids
        .iter()
        .rposition(|&v| v == 47 || v == 92 || v == 58)
        .map_or(ids, |i| &ids[i + 1..]);
    let ext = base
        .iter()
        .rposition(|&v| v == 46)
        .map_or(&[][..], |i| &base[i + 1..]);
    let mut out = prefix.to_vec();
    let suffix =
        suffix.ok_or("Model hardware path actual current RendererHBPPSuffix is unbound")?;
    out.extend_from_slice(suffix);
    if !ext.is_empty() {
        out.push(46);
        out.extend_from_slice(ext);
    }
    Ok(out)
}
#[test]fn all_exact_native_path_rows(){
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name//winter//model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder//winter//");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-h.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-custom.tga");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-h");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-custom");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-h");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-custom");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-h.nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-h");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-custom");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name//winter//model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder//winter//");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-h.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-custom.tga");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-h");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-custom");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-h");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-custom");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-h.nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-h");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-custom");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//EnvironmentMap.tga");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art/Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units/winter/gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"Art\\Units//winter//gbr_corps.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C:/winter/model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"C://winter//model.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name/winter/model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"dir.name//winter//model");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"foo.nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//foo.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&"noext".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//noext");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"/winter/.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&".nif".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"//winter//.nif");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"winter/".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"/winter".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder/winter/");
assert_eq!(String::from_utf16(&subfolder_candidate(&"folder/".encode_utf16().collect::<Vec<_>>(),&"//winter//".encode_utf16().collect::<Vec<_>>()).unwrap()).unwrap(),"folder//winter//");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-h.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap.tga");
assert_eq!(String::from_utf16(&model_variant(&"EnvironmentMap.tga".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"EnvironmentMap-custom.tga");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art/Units/gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art/Units/gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps.nif");
assert_eq!(String::from_utf16(&model_variant(&"Art\\Units\\gbr_corps.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"Art\\Units\\gbr_corps-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model.nif");
assert_eq!(String::from_utf16(&model_variant(&"C:/model.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"C:/model-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"dir.name/model");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-h");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir");
assert_eq!(String::from_utf16(&model_variant(&"dir.name/model".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"dir-custom");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-h.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo.nif");
assert_eq!(String::from_utf16(&model_variant(&"foo.nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"foo-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-h");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext");
assert_eq!(String::from_utf16(&model_variant(&"noext".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"noext-custom");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-h.nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),".nif");
assert_eq!(String::from_utf16(&model_variant(&".nif".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"-custom.nif");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),false,None).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-h".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-h");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/");
assert_eq!(String::from_utf16(&model_variant(&"folder/".encode_utf16().collect::<Vec<_>>(),true,Some(&"-custom".encode_utf16().collect::<Vec<_>>())).unwrap()).unwrap(),"folder/-custom");
}
//! Pure source inputs for the ONE current startup SVar owner.
//!
//! These temporary values are not another mutable SVar State. The caller
//! applies them to load_view_variables::State at its existing reset/publication
//! points. Validation is preflight, not native exception-prefix emulation.
use crate::{catalog::Catalog,saved_action_settings};
use aa_mapworld::WorldInventory;
use aa_numeric::{Binary,Precision};
use aa_rules::{actors::ResolvedNode,native::TypedValue};

pub(crate) struct SourceValues {
    pub stockpile_default110:u32,
    pub city_victory_ratio130:i32,
    pub stockpile_maximum1b8:u32,
    pub starting_experience134:u32,
}
pub(crate) struct PublishedValues {
    pub stockpile_default110:u32,
    pub city_victory_ratio130:i32,
    pub starting_experience134:u32,
}
fn err(e:impl std::fmt::Display)->String {e.to_string()}
fn text(node:&ResolvedNode<'_>,name:&str,pc:Precision)->Result<String,String> {
    match node.field(name,pc).map_err(err)? {
        Some(TypedValue::String{value,..})=>Ok(value),
        _=>Err(format!("mission WorldRule {name} requires its actual String field")),
    }
}

fn definition<'a>(catalog:&'a Catalog<'_>,name:&str,ty:&str,pc:Precision)->Result<(ResolvedNode<'a>,bool),String> {
    let registry=catalog.actors.native_data_registry("WorldRule").ok_or("mission WorldRule registry absent")?;
    let def=registry.find(name).ok_or_else(||format!("mission WorldRule {name} absent"))?;
    if !def.ids.eq_ignore_ascii_case(name) {return Err("mission WorldRule translated SVar identity is unbound".into())}
    let node=catalog.actors.native_data_node(def).map_err(err)?;
    if !text(&node,"type",pc)?.eq_ignore_ascii_case(ty) {
        return Err(format!("mission WorldRule {name} has an unbound native type"));
    }
    let editable=node.field("always_editable",pc).map_err(err)?.ok_or("mission always_editable source absent")?
        .boolean().map_err(err)?;
    Ok((node,editable))
}

fn validate_svar(catalog:&Catalog<'_>,name:&str,fixed:bool,pc:Precision)->Result<(),String> {
    let rows=catalog.settings.as_ref().ok_or("mission Settings owner absent")?.declarations
        .get(&name.to_ascii_lowercase()).ok_or("mission source SVar declaration absent")?;
    let [row]=rows.as_slice() else {return Err("mission source SVar override order is unbound".into())};
    if !row.field_type.as_deref().is_some_and(|ty|ty.eq_ignore_ascii_case(if fixed {"fixed"}else{"int"})) {
        return Err("mission source SVar type differs from native constructor".into());
    }
    let mut matched=0;
    for node in catalog.actors.resolved_definitions("SVars").map_err(err)? {
        match node.field(name,pc).map_err(err)? {
            Some(TypedValue::Scalar(v)) if fixed && v.source==row.source
                && v.field_type==aa_rules::native::ScalarType::Fixed
                && v.bits==aa_rules::native::scalar_literal(&row.raw,pc).map_err(err)? =>matched+=1,
            Some(TypedValue::Integer{value,source,..}) if !fixed && source==row.source
                && value==row.raw.trim().parse::<i32>().map_err(err)? =>matched+=1,
            _=>{},
        }
    }
    if matched!=1 {return Err("mission public Settings projection differs from private registered source value/span".into())}
    Ok(())
}

/// Integer literal scope is deliberate: current fixed WorldRule default500 is
/// exactly representable. Arbitrary CRT decimal-string conversion is not
/// silently replaced with a host-language floating parser.
fn default_bits(node:&ResolvedNode<'_>,pc:Precision)->Result<u32,String> {
    let value=text(node,"default",pc)?.trim().parse::<i32>().map_err(|_|"mission fixed default requires the proven signed integer literal scope")?;
    Binary::from_i32(value).store_f32().map_err(err)
}

pub(crate) fn source_reset(catalog:&Catalog<'_>,pc:Precision)->Result<SourceValues,String> {
    catalog.require_source_owner()?;
    validate_svar(catalog,"ResourceStockpileDefault",true,pc)?;
    validate_svar(catalog,"CityVictoryRatio",false,pc)?;
    validate_svar(catalog,"EconomyStockpileResourceMax",true,pc)?;
    //507FB9..507FE7 registers StartingExperience at index4D (cache+134)
    //as Fixed/flags0/initial0 even when no TGI declaration overrides it.
    //474C50 applies the WorldRule default200 before source snapshot/reset.
    let mut declared_experience=false;
    for node in catalog.actors.resolved_definitions("SVars").map_err(err)? {
        declared_experience|=node.field("StartingExperience",pc).map_err(err)?.is_some();
    }
    let public_experience=catalog.settings.as_ref().ok_or("mission Settings owner absent")?
        .declarations.contains_key("startingexperience");
    if declared_experience||public_experience {validate_svar(catalog,"StartingExperience",true,pc)?;}
    let registry=catalog.actors.native_data_registry("WorldRule").ok_or("mission WorldRule registry absent")?;
    if registry.find("EconomyStockpileResourceMax").is_some() {
        return Err("mission EconomyStockpileResourceMax WorldRule default/publication is unbound".into());
    }
    let stockpile_maximum1b8=catalog.settings.as_ref().ok_or("mission Settings owner absent")?
        .unique_fixed("EconomyStockpileResourceMax",pc).map_err(err)?.0;
    let (stock,_)=definition(catalog,"ResourceStockpileDefault","fixed",pc)?;
    let (city,_)=definition(catalog,"CityVictoryRatio","int",pc)?;
    let (experience,_)=definition(catalog,"StartingExperience","fixed",pc)?;
    Ok(SourceValues{stockpile_default110:default_bits(&stock,pc)?,
        city_victory_ratio130:text(&city,"default",pc)?.trim().parse::<i32>().map_err(err)?,stockpile_maximum1b8,
        starting_experience134:default_bits(&experience,pc)?})
}

pub(crate) fn publication(catalog:&Catalog<'_>,inventory:&WorldInventory,pc:Precision)->Result<PublishedValues,String> {
    let reset=source_reset(catalog,pc)?;
    let rules=saved_action_settings::rules(inventory)?;
    saved_action_settings::game_type(catalog,&rules)?;
    let mode=catalog.native_load_mode.ok_or("mission WorldRules requires actual loading mode")?;
    if !matches!(mode,1|2|4) {return Err("mission WorldRules loading mode is unbound".into())}
    let (_,stock_editable)=definition(catalog,"ResourceStockpileDefault","fixed",pc)?;
    let (_,city_editable)=definition(catalog,"CityVictoryRatio","int",pc)?;
    let (_,experience_editable)=definition(catalog,"StartingExperience","fixed",pc)?;
    let registry=catalog.actors.native_data_registry("WorldRule").ok_or("mission WorldRule registry absent")?;
    let stock_def=registry.find("ResourceStockpileDefault").ok_or("mission stockpile definition absent")?;
    let city_def=registry.find("CityVictoryRatio").ok_or("mission city definition absent")?;
    let experience_def=registry.find("StartingExperience").ok_or("mission experience definition absent")?;
    let mut stock=reset.stockpile_default110;let mut city=reset.city_victory_ratio130;
    let mut experience=reset.starting_experience134;
    for row in &rules.rules {
        let n=row.key.iter().position(|&u|u==0).unwrap_or(row.key.len());
        let name=String::from_utf16(&row.key[..n]).map_err(err)?;
        let Some(found)=registry.find(&name) else {continue};
        if found.path==stock_def.path && row.kind==2 {stock=row.value;}
        if found.path==city_def.path && row.kind==1 {city=row.value as i32;}
        if found.path==experience_def.path && row.kind==2 {experience=row.value;}
    }
    if mode&2!=0 {
        if stock_editable {stock=reset.stockpile_default110;}
        if city_editable {city=reset.city_victory_ratio130;}
        if experience_editable {experience=reset.starting_experience134;}
    }
    // Actual source-created type2/type3 variables have flags0. Slider limits
    // belong to the UI descriptor; neither setter clamps these current words.
    Ok(PublishedValues{stockpile_default110:stock,city_victory_ratio130:city,starting_experience134:experience})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore="requires AA_GAME_ROOT actual source and maps"]
    fn actual_source_and_three_map_rule_words_all_precision() {
        use aa_rules::sources::{SourceLimits,V14Sources};
        let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
        let mut catalog=Catalog::new(&actors);
        catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,aa_rules::tgi::Limits::default()).unwrap());
        catalog.native_load_mode=Some(1);
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-mission-variables.json")).unwrap();
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            let source=source_reset(&catalog,pc).unwrap();
            assert_eq!(source.stockpile_default110,500f32.to_bits());
            assert_eq!(source.city_victory_ratio130,0);
            assert_eq!(source.stockpile_maximum1b8,99999f32.to_bits());
            assert_eq!(source.starting_experience134,200f32.to_bits());
            for name in ["Campaign/tutorial02","DeveloperMaps/(3v3)Firefight","(2v2)Berlin"] {
                let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(root.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
                let inventory=WorldInventory::decode(&map,u64::MAX).unwrap();
                let result=publication(&catalog,&inventory,pc).unwrap();
                let pc_name=format!("{pc:?}");
                let row=fixture["cases"].as_array().unwrap().iter().find(|row|row["precision"]==pc_name && row["input"]["actual_map"]==name).unwrap();
                let stock=row["output"]["current"]["ResourceStockpileDefault"].as_u64().unwrap() as u32;
                let city=row["output"]["current"]["CityVictoryRatio"].as_u64().unwrap() as u32 as i32;
                assert_eq!(result.stockpile_default110,stock,"{name} {pc:?}");
                assert_eq!(result.city_victory_ratio130,city,"{name} {pc:?}");
                let section=inventory.opaque_initialization.iter().find(|v|v.name=="RULES").unwrap();
                assert_eq!(row["input"]["rules_sha256"],section.source.sha256);
                println!("{name} {pc:?} stock={stock:08x} city={city}");
            }
        }
        catalog.settings.as_mut().unwrap().declarations.get_mut("resourcestockpiledefault").unwrap()[0].raw="1234".into();
        assert!(source_reset(&catalog,Precision::Double53).is_err());
    }
}

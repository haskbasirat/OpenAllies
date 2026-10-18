//!73A560's exact lazy predicate. Field providers are supplied at each native
//!read; modern rendering supplies its explicit architecture mapping separately.
type Result<T>=std::result::Result<T,String>;
pub(super) trait Fields{
    fn receiver_191(&mut self)->Result<u8>;
    fn problematic_driver_275(&mut self)->Result<u8>;
    fn uvar_113(&mut self)->Result<u8>;
    fn lit_bone_shader_128(&mut self)->Result<bool>;
    fn unlit_bone_shader_12c(&mut self)->Result<bool>;
}
pub(super) fn use_hardware_model(fields:&mut impl Fields)->Result<bool>{
    if fields.receiver_191()?==0{return Ok(false)}
    if fields.problematic_driver_275()?!=0{return Ok(false)}
    if fields.uvar_113()?==0{return Ok(false)}
    if !fields.lit_bone_shader_128()?{return Ok(false)}
    if !fields.unlit_bone_shader_12c()?{return Ok(false)}
    Ok(true)
}
#[cfg(test)]mod tests{
    use super::*;
    struct Read<'a>{input:&'a serde_json::Value,reads:Vec<&'static str>}
    impl Read<'_>{fn word(&mut self,key:&'static str)->u8{self.reads.push(key);self.input[key].as_u64().unwrap()as u8}}
    impl Fields for Read<'_>{
        fn receiver_191(&mut self)->Result<u8>{Ok(self.word("receiver191"))}
        fn problematic_driver_275(&mut self)->Result<u8>{Ok(self.word("compat275"))}
        fn uvar_113(&mut self)->Result<u8>{Ok(self.word("uvar113"))}
        fn lit_bone_shader_128(&mut self)->Result<bool>{Ok(self.word("lit128")!=0)}
        fn unlit_bone_shader_12c(&mut self)->Result<bool>{Ok(self.word("unlit12c")!=0)}
    }
    #[test]fn exact_model_predicate_original_v14_all_pc_order_and_ctor_fields(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/model-shader-policy/native-model-policy.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),648);
        for row in rows{
            let mut fields=Read{input:&row["input"],reads:Vec::new()};
            assert_eq!(use_hardware_model(&mut fields).unwrap(),row["output"]["result"]==1);
            assert_eq!(serde_json::json!(fields.reads),row["output"]["reads"]);
        }
        let ctor=fixture["constructors"].as_array().unwrap();assert_eq!(ctor.len(),18);
        for row in ctor{
            assert_eq!(row["receiver191"],row["skinned"]);assert_eq!(row["lit128"],0);assert_eq!(row["unlit12c"],0);
            assert_eq!(row["published_skinned"],row["skinned"]!=0);
            assert_eq!(row["published_unskinned"],row["skinned"]==0);
        }
    }
    #[test]fn required_reached_current_reader_is_not_replaced_by_absent_shader_pair(){
        struct Required;impl Fields for Required{
            fn receiver_191(&mut self)->Result<u8>{Ok(1)}
            fn problematic_driver_275(&mut self)->Result<u8>{Err("current compatibility owner required".into())}
            fn uvar_113(&mut self)->Result<u8>{panic!("unreached")}
            fn lit_bone_shader_128(&mut self)->Result<bool>{Ok(false)}
            fn unlit_bone_shader_12c(&mut self)->Result<bool>{panic!("unreached")}
        }
        assert_eq!(use_hardware_model(&mut Required).unwrap_err(),"current compatibility owner required");
    }
}

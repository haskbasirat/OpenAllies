use aa_orders::{Error,Result,payload::{self,Field,NameKind,Registry,Resolver,SchemaContext,ThingCheck}};
struct Context;
impl SchemaContext for Context{
    fn registry_count(&self,_:Registry)->Result<usize>{Ok(39)}
    fn team_command_flags(&self,_:u32)->Result<u32>{unreachable!()}
    fn definition_kind(&self,_:Registry,_:u32)->Result<u32>{unreachable!()}
    fn zero_width_list_budget(&self)->usize{0}
}
struct Unbound;
impl Resolver for Unbound{
    fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{panic!("unbound must not become nativeNULL")}
    fn player(&self,_:u32)->bool{panic!("unbound must not become nativeNULL")}
    fn definition(&self,_:Registry,_:u32)->bool{panic!("unbound must not become count sentinel")}
    fn named(&self,_:NameKind,_:&[u16])->Option<Vec<u16>>{panic!("unbound must retain required boundary")}
    fn try_thing(&self,_:ThingCheck,_:u16)->Result<Option<u16>>{Err(Error::SchemaContext("actual Thing owner required"))}
    fn try_player(&self,_:u32)->Result<bool>{Err(Error::SchemaContext("actual Player owner required"))}
    fn try_definition(&self,_:Registry,_:u32)->Result<bool>{Err(Error::SchemaContext("actual registry owner required"))}
    fn try_named(&self,_:NameKind,_:&[u16])->Result<Option<Vec<u16>>>{Err(Error::SchemaContext("actual named owner required"))}
}
#[test]
fn unbound_domains_fail_instead_of_serializing_native_null_sentinels(){
    for field in [Field::Thing{check:ThingCheck::B4,slot:7},Field::Player(7),Field::Def{registry:Registry(0xb3aa58),index:3},Field::Named{kind:NameKind::RandomMapConfig,units:vec![65,0,66]}]{
        let input=vec![Field::Uint{width:32,value:17},field];let before=input.clone();
        assert!(payload::normalize_for_decree(&input,&Unbound,&Context).is_err());
        assert_eq!(input,before);
    }
}
struct Missing;
impl Resolver for Missing{
    fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{None}
    fn player(&self,_:u32)->bool{false}
    fn definition(&self,_:Registry,_:u32)->bool{false}
    fn named(&self,_:NameKind,_:&[u16])->Option<Vec<u16>>{None}
}
#[test]
fn existing_resolvers_keep_genuinely_missing_reference_normalization(){
    let input=[Field::Thing{check:ThingCheck::B4,slot:7},Field::Player(7),Field::Def{registry:Registry(0xb3aa58),index:3}];
    assert_eq!(payload::normalize_for_decree(&input,&Missing,&Context).unwrap(),[Field::Thing{check:ThingCheck::B4,slot:0},Field::Player(0),Field::Def{registry:Registry(0xb3aa58),index:39}]);
}

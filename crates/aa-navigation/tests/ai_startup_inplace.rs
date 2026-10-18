use aa_navigation::{ai_startup::{self,Initializing,Inputs},sai_state::{self,Kingdom},startup::Object,topology::{Topology,GridLayout}};
use serde_json::{Value,json};
fn setup(c:&Value)->(Inputs,aa_navigation::region_search::RegionGraph,sai_state::State) {
    let v=&c["layout"];let bytes=c["input_hex"].as_str().unwrap().as_bytes().chunks_exact(2)
        .map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect::<Vec<_>>();
    let (topology,_)=Topology::decode_prefix(&bytes,GridLayout {width:v["width"].as_i64().unwrap()as i32,
        height:v["height"].as_i64().unwrap()as i32,coordinate_shift:v["coordinate_shift"].as_u64().unwrap()as u32,
        extent_bits:std::array::from_fn(|i|(v["extent_i32"][i].as_i64().unwrap()as f32).to_bits())},28).unwrap();
    let graph=topology.prepare_graph().unwrap();let kingdoms:Vec<Option<Kingdom>>=serde_json::from_value(c["kingdoms"].clone()).unwrap();
    let mut state=sai_state::State::new(graph.nodes.len()).unwrap();
    for k in kingdoms.iter().flatten() {
        state.register_kingdom_regions(*k,&topology,&vec![sai_state::TileKnowledge {word14:0,word18:0};(topology.layout.width*topology.layout.height)as usize],
            sai_state::VisibilitySettings {fog_enabled:false,visible_bit_offset:16}).unwrap();
    }
    (Inputs {controller:Object(100),player:Object(200),player_index:c["player_index"].as_i64().unwrap()as i32,kingdoms,
        own_slot:c["own_slot"].as_u64().unwrap()as usize,registered_ids:serde_json::from_value(c["registered_ids"].clone()).unwrap(),
        ability_counts:serde_json::from_value(c["ability_counts"].clone()).unwrap(),script_count:c["script_count"].as_u64().unwrap()as usize,
        neighbor_depth:c["neighbor_depth"].as_i64().unwrap()as i32,recruitment:serde_json::from_value(c["recruitment"].clone()).unwrap(),
        owned_units:serde_json::from_value(c["owned_units"].clone()).unwrap()},graph,state)
}
fn snapshot(p:&Initializing)->Value {
    let controller=p.controller.as_ref().unwrap();
    let rows=p.region_players.iter().map(|r|json!({"region":r.region,"player":r.player,"land":r.land,"water":r.water,"word34":r.word34,
        "arrays":r.arrays.iter().map(Vec::len).collect::<Vec<_>>()})).collect::<Vec<_>>();
    json!({"ready":p.ready,"script_words":p.script_words,"word10":p.word10,"word14":p.word14,"pending_count":p.pending.len(),
        "groups":controller.groups,"goals":p.goals,"region_kingdoms":p.region_kingdoms,"region_players":rows})
}
fn fixture()->Value {
    serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-sai-world-ready/native-ai-startup.json")).unwrap()).unwrap()
}
#[test]
fn full_initializer_replays_all_paired_native_cases_in_place() {
    let f=fixture();assert_eq!(f["rows"].as_array().unwrap().len(),54);
    for row in f["rows"].as_array().unwrap() {
        let (inputs,graph,regions)=setup(&row["input"]);let mut state=Initializing::from_native_constructor().unwrap();
        ai_startup::initialize_in_place(&inputs,&graph,&regions,&mut state).unwrap();
        assert_eq!(snapshot(&state),row["output"],"{} {}",row["precision"],row["input"]["name"]);
    }
}
#[test]
fn neighbor_error_retains_published_controller_goals_and_script_owner() {
    let f=fixture();assert_eq!(f["prefix_rows"].as_array().unwrap().len(),3);
    for row in f["prefix_rows"].as_array().unwrap() {
        let (inputs,mut graph,regions)=setup(&row["input"]);assert!(inputs.neighbor_depth>0);
        // Explicit malformed graph probe stops before any first neighbor link.
        // Native counterpart is a controlled instruction-entry cutoff, not a
        // claimed native throw or acceptance of malformed graph data.
        graph.nodes[0].neighbors=vec![u32::MAX];
        let mut state=Initializing::from_native_constructor().unwrap();
        assert!(ai_startup::initialize_in_place(&inputs,&graph,&regions,&mut state).is_err());
        assert!(state.attempted()&&!state.ready&&state.controller.is_some()&&!state.goals.is_empty());
        assert_eq!(snapshot(&state),row["output"]);
        let before=snapshot(&state);
        assert!(ai_startup::initialize_in_place(&inputs,&graph,&regions,&mut state).is_err());
        assert_eq!(snapshot(&state),before,"retry cannot reset or turn partial startup into success");
    }
}
struct SourceStop {input:Inputs,site:String,calls:Vec<&'static str>}
impl ai_startup::StartupData for SourceStop {
    fn identities(&mut self)->sai_state::Result<ai_startup::Identities> {self.calls.push("identities");self.input.identities()}
    fn script_count(&mut self)->sai_state::Result<usize> {self.calls.push("scripts");self.input.script_count()}
    fn ability_counts(&mut self)->sai_state::Result<[i32;3]> {self.calls.push("ability");if self.site=="ability" {Err(sai_state::Error::Invalid("explicit unbound source"))}else {self.input.ability_counts()}}
    fn recruitment(&mut self)->sai_state::Result<Option<ai_startup::Recruitment>> {self.calls.push("recruitment");if self.site=="recruitment" {Err(sai_state::Error::Invalid("explicit unbound source"))}else {self.input.recruitment()}}
    fn owned_units(&mut self)->sai_state::Result<Vec<Option<Object>>> {self.calls.push("owned_units");if self.site=="owned_units" {Err(sai_state::Error::Invalid("explicit unbound source"))}else {self.input.owned_units()}}
    fn neighbor_depth(&mut self)->sai_state::Result<i32> {self.calls.push("neighbors");self.input.neighbor_depth()}
}
#[test]
fn reached_source_refusals_keep_the_exact_native_publication_prefix() {
    let f=fixture();assert_eq!(f["source_prefix_rows"].as_array().unwrap().len(),9);
    for row in f["source_prefix_rows"].as_array().unwrap() {
        let (inputs,graph,regions)=setup(&row["input"]);let site=row["cutoff"].as_str().unwrap();
        let mut source=SourceStop {input:inputs,site:site.into(),calls:Vec::new()};
        let mut state=Initializing::from_native_constructor().unwrap();
        assert!(ai_startup::initialize_from_source(&mut source,&graph,&regions,&mut state).is_err());
        let c=state.controller.as_ref().unwrap();
        let got=json!({"ready":state.ready,"controller_present":true,"script_words":state.script_words,
            "words10_14":[state.word10,state.word14],"region_count":state.region_kingdoms.len(),
            "pending_kinds":state.pending.iter().map(|&i|state.goals[i].kind).collect::<Vec<_>>(),
            "group_counts":c.groups.iter().map(|g|g.iter().map(Vec::len).collect::<Vec<_>>()).collect::<Vec<_>>()});
        assert_eq!(got,row["output"],"{site} {}",row["precision"]);
        assert_eq!(source.calls.last(),Some(&site));
        let calls=source.calls.clone();
        assert!(ai_startup::initialize_from_source(&mut source,&graph,&regions,&mut state).is_err());
        assert_eq!(source.calls,calls,"failed retry does not reread/reset any actual source or owner");
    }
}

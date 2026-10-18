//!The application B39690+84 random owner, distinct from UI+D4 and simulation.
//!41788C..9C first constructs85D870 on the same App+84 word.419898..AD
//!writes85D880 from actual time32(NULL).5C8650 draws this same
//!owner once before reseeding UI+D4. Other native random consumers remain
//!separate reached ports; this is not whole Application::Init parity.
use aa_simulation::NativeRng;
use std::{sync::{Arc,Weak},thread::ThreadId,time::{SystemTime,UNIX_EPOCH}};
type Result<T>=std::result::Result<T,String>;
pub struct State{rng:NativeRng,time32:i32,thread:ThreadId,ui:Option<Weak<()>>}
impl State{
    ///One real UTC time32 query at application startup. There is no raw seed,
    ///Default, Clone or per-Actor constructor. An unsupported clock range is
    ///an error before any UI reseed, never a default time or simulation seed.
    pub fn from_application_time32()->Result<Self>{
        //The constructor word exists before the later time32 seed writer;
        //its fixed value is never exposed as the initialized visual stream.
        let mut rng=NativeRng::default();
        let now=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?;
        let time32=i32::try_from(now.as_secs()).map_err(|_|"application UTC clock is outside native time32 domain")?;
        rng.reseed(time32 as u32);
        Ok(Self{rng,time32,thread:std::thread::current().id(),ui:None})
    }
    pub fn seed_ui_5c8650(&mut self,ui:&mut crate::ui::Ui)->Result<()>{
        if self.thread!=std::thread::current().id(){return Err("application/UI RNG writer is on another host thread".into())}
        if ui.random_initialized{return Err("same UI5C8650 random reseed already ran".into())}
        if self.ui.as_ref().and_then(Weak::upgrade).is_some_and(|current|!Arc::ptr_eq(&current,&ui.random_identity)){
            return Err("application RNG is bound to another live UI allocation".into())
        }
        //Native unsigned remainder is intentional, including MAX ->0. This
        //does not use next_bounded rejection or the simulation's random state.
        let seed=self.rng.next_u32()%u32::MAX;
        ui.random_d4.reseed(seed);ui.random_initialized=true;
        self.ui=Some(Arc::downgrade(&ui.random_identity));Ok(())
    }
    pub fn startup_time32(&self)->i32{self.time32}
}

#[cfg(test)]mod tests{
    use super::*;
    fn fixture()->serde_json::Value{
        serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-application-random.json")).unwrap()
    }
    fn word(value:&serde_json::Value)->u32{value.as_u64().unwrap().try_into().unwrap()}
    #[test]
    fn native_application_and_ui_seed_streams(){
        let fixture=fixture();let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),108);
        for row in rows{
            let output=&row["output"];
            let mut application=NativeRng::default();assert_eq!(application.state,word(&output["application_constructor"]));
            application.reseed(row["input"]["time32"].as_i64().unwrap()as u32);
            assert_eq!(application.state,word(&output["application_seeded"]));
            for prior in output["preceding"].as_array().unwrap(){assert_eq!(application.next_u32(),word(&prior["value"]));assert_eq!(application.state,word(&prior["state"]));}
            let seed=application.next_u32()%u32::MAX;
            assert_eq!(application.state,word(&output["global_after"]));
            let mut ui=NativeRng::default();assert_eq!(ui.state,word(&output["ui_constructor"]));ui.reseed(seed);
            assert_eq!(ui.state,word(&output["ui_seeded"]));
            for draw in output["values"].as_array().unwrap(){assert_eq!(ui.next_u32(),word(&draw["value"]));assert_eq!(ui.state,word(&draw["state"]));}
            assert_eq!(application.state,word(&output["global_after"]));
        }
    }
    #[test]
    fn actual_application_clock_seed_is_retained(){
        let before=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let owner=State::from_application_time32().unwrap();
        let after=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert!((before..=after).contains(&(owner.startup_time32()as u64)));
        assert_eq!(owner.rng,NativeRng::from_native_seed(owner.startup_time32()as u32));
        assert!(owner.ui.is_none());
    }
    #[test]
    #[ignore="loads actual UI source data; no native window, game or GPU"]
    fn actual_ui_allocation_reseed_and_retained_draws(){
        let fixture=fixture();
        for row in fixture["rows"].as_array().unwrap().iter().filter(|r|r["image"]=="original"&&r["precision"]=="Single"){
            let time32=row["input"]["time32"].as_i64().unwrap()as i32;
            //This explicitly labelled native-input fixture injects a laboratory
            //time32 only inside this private test. The production factory always
            //queries the OS; neither it nor State exposes a raw-seed constructor.
            let mut owner=State{rng:NativeRng::default(),time32,thread:std::thread::current().id(),ui:None};
            owner.rng.reseed(time32 as u32);
            for _ in 0..row["input"]["prior_global_draws"].as_u64().unwrap(){owner.rng.next_u32();}
            let mut ui=crate::ui::Ui::new(crate::fs::DataRoot::workspace_default(),crate::layout::Screen{width:1280,height:720});
            assert!(ui.next_view_random_word_d4().is_err());owner.seed_ui_5c8650(&mut ui).unwrap();
            assert_eq!(owner.rng.state,word(&row["output"]["global_after"]));
            assert_eq!(ui.random_d4.state,word(&row["output"]["ui_seeded"]));
            let committed=owner.rng.state;assert!(owner.seed_ui_5c8650(&mut ui).is_err());assert_eq!(owner.rng.state,committed);
            for(i,draw)in row["output"]["values"].as_array().unwrap().iter().enumerate(){
                if i==4{ui.clear();}
                assert_eq!(ui.next_view_random_word_d4().unwrap(),word(&draw["value"]));assert_eq!(ui.random_d4.state,word(&draw["state"]));
            }
            assert_eq!(owner.rng.state,committed);
            let mut foreign=crate::ui::Ui::new(crate::fs::DataRoot::workspace_default(),crate::layout::Screen{width:1280,height:720});
            assert!(owner.seed_ui_5c8650(&mut foreign).is_err());assert_eq!(owner.rng.state,committed);assert!(!foreign.random_initialized);
        }
    }
}

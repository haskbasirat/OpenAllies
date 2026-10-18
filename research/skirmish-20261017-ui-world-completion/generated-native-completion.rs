type Result<T>=std::result::Result<T,String>;
fn missing(v:impl Into<String>)->String{v.into()}
trait CompletionWorld {
    fn session_diagnostic(&mut self,success:bool)->Result<()>;
    fn store_status(&mut self,status:u32)->Result<()>;
    fn admin_diagnostic(&mut self,success:bool)->Result<()>;
    fn emit_status(&mut self)->Result<()>;
}
fn complete(world:&mut impl CompletionWorld,success:bool)->Result<()> {
    world.session_diagnostic(success)?;
    world.store_status(if success {3}else{4})?;
    world.admin_diagnostic(success)?;
    world.emit_status()
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value,json};
    struct Replay<'a>{input:&'a Value,status:u32,trace:Vec<Value>}
    impl Replay<'_>{fn stop(&self,label:&str)->Result<()>{if self.input["fail"].as_str()==Some(label){Err(missing("required child cutoff"))}else{Ok(())}}}
    impl CompletionWorld for Replay<'_>{
        fn session_diagnostic(&mut self,_:bool)->Result<()>{self.trace.push(json!("session_diagnostic"));self.stop("session_diagnostic")}
        fn store_status(&mut self,status:u32)->Result<()>{self.status=status;Ok(())}
        fn admin_diagnostic(&mut self,_:bool)->Result<()>{self.trace.push(json!("admin_diagnostic"));self.stop("admin_diagnostic")}
        fn emit_status(&mut self)->Result<()>{self.trace.push(json!(["emit",self.status]));self.stop("emit")}
    }
    #[test]
    fn native_vworld_complete_store_precedes_admin_diagnostic_and_request() {
        let fixture:Value=serde_json::from_str(include_str!("native-loading.json")).unwrap();let mut checked=0;
        for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];if input["op"]!="vworld_status"{continue;}
            let mut world=Replay{input,status:input["old"].as_u64().unwrap()as u32,trace:Vec::new()};let result=complete(&mut world,input["success"].as_u64().unwrap()!=0);
            assert_eq!(json!({"status":world.status,"trace":world.trace,"error":if result.is_err(){Some("required child cutoff")}else{None}}),row["output"],"{input}");checked+=1;
        }
        assert_eq!(checked,216);
    }
}

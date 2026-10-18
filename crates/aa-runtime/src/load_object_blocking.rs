//! Actual4EB6E0 blocking notification: query once, then refresh discovered
//! Actor owners in order. Both current and child shells stay in their actual
//! allocations, outside distinct matching Loading registry entries.
use crate::{catalog::Catalog,live_object_init::{Record,Source},
    load_actor_overlaps::{self,QueryContext},load_placement_children::Policy,
    load_spatial::HeldShell};
use aa_actors::{ActorId,actor_creation::{Error,Result}};
use aa_mapworld::restore::RestoredRecord;
use aa_numeric::Precision;

fn error(e:impl std::fmt::Display)->Error { Error::World(e.to_string()) }

pub(crate) fn notify(
    catalog:&Catalog<'_>,record:&mut Record<'_>,context:&mut QueryContext<'_>,pc:Precision,
    world_time:impl Fn()->Result<u32>,
) -> Result<()> {
    let source=Source::bind(catalog,record.definition)?;
    source.validate_record(record)?;
    if record.shell.thing.flags&(1<<10)==0 { return Ok(()) }
    let geometry=context.geometry(pc)?;
    let fields=context.source_fields;
    let discovered=context.spatial.with_readonly_query(catalog,record,|index,bindings,current| {
        let key=bindings.key(current.owner).map_err(|e|aa_spatial::Error::World(e.to_string()))?;
        let mut policy=Policy::new(catalog,current,fields,bindings);
        let mut query=aa_spatial::actor_context::prepare(index,key,false,&policy)?;
        //4EB733/4EB737 write descriptor+1C/+20 after its constructor.
        query.descriptor.source_actor=None;
        query.descriptor.ignore_actor=Some(key);
        aa_spatial::overlaps::all_overlaps(index,&geometry,&mut query,true,&mut policy)
    }).map_err(error)?;
    for raw in discovered {
        let id=ActorId::from_raw(raw);
        if id==record.owner {
            // Normally excluded above; preserve the real dispatch if reached.
            load_actor_overlaps::refresh_query(catalog,record,context,pc,&world_time)?;
            continue;
        }
        let class=match record.registry.get(id).map(|e|&e.value) {
            Some(RestoredRecord::Actor(v))|Some(RestoredRecord::Object(v))|Some(RestoredRecord::Thing(v))=>v.definition.native_class,
            Some(RestoredRecord::Kingdom(v))=>v.runtime.as_ref().ok_or(Error::Unsupported("4EB6E0 actual discovered Kingdom runtime"))?.definition.native_class,
            _=>return Err(Error::NullDereference("4EB6E0 discovered GTID generation/virtual+B4")),
        };
        match class {
            2|6|7|9=>continue, //actual constant-false virtual+B4
            3|4=>(),          //actual constant-true virtual+B4
            _=>return Err(Error::Unsupported("4EB6E0 discovered virtual+B4 class")),
        }
        let mut held=std::mem::replace(record.registry.value_mut(id).unwrap(),RestoredRecord::Loading);
        let outcome=(|| {
            let (shell,definition)=match &mut held {
                RestoredRecord::Actor(v)=>(&mut v.shell,&v.definition),
                RestoredRecord::Kingdom(v)=> {
                    let v=v.runtime.as_mut().ok_or(Error::Unsupported("4EB6E0 held Kingdom runtime"))?;
                    (&mut v.base,&v.definition)
                }
                _=>return Err(Error::Unsupported("4EB6E0 actual Actor allocation differs")),
            };
            let mut extra=HeldShell {owner:record.owner,shell:&mut *record.shell,definition:record.definition};
            let mut child=Record {owner:id,shell,definition,registry:&mut *record.registry};
            load_actor_overlaps::refresh_with_extra(catalog,&mut child,context,pc,&world_time,&mut extra).map(|_|())
        })();
        // Child effects and the same allocation survive errors. No fallible
        // return is allowed while its registry entry is still Loading.
        *record.registry.value_mut(id).expect("held child generation retained")=held;
        outcome?;
    }
    Ok(())
}

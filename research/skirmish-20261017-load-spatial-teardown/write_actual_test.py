"""Create this new focused test using the existing genuine-source setup recipe."""
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
old = (ROOT / "crates/aa-runtime/src/load_spatial_tests.rs").read_text()
setup = old[old.index("    use crate::{"):old.index("    let mut cases = Vec::new();")]
loop = old[old.index("    for pc in ["):old.index("        // Source authority is independent")]
tail = r'''
        spatial.refresh_membership(&catalog, &mut record).unwrap();
        let key = spatial.key(id).unwrap();
        drop(record);
        *attempt.registry.value_mut(id).unwrap() = held;
        spatial.index.take_events();
        let before = snapshot(&spatial.index);
        let flags;
        {
            //Actual allocation stays in its registered entry during teardown.
            let RestoredRecord::Kingdom(kingdom) = attempt.registry.value_mut(id).unwrap() else { unreachable!() };
            let runtime = kingdom.runtime.as_mut().unwrap();
            let address = std::ptr::from_ref(&runtime.base);
            let components = runtime.base.actor.as_ref().unwrap().components.iter()
                .map(|(s,c)|(s,std::ptr::from_ref(c))).collect::<Vec<_>>();
            let wrong_pc = if pc == Precision::Single24 {Precision::Extended64} else {Precision::Single24};
            assert!(spatial.detach_for_destruction(&catalog,id,&mut runtime.base,&runtime.definition,wrong_pc).is_err());
            assert_eq!(snapshot(&spatial.index), before);
            let original_pc = spatial.bindings[0].source_precision;
            spatial.bindings[0].source_precision = Precision::Single24;
            assert!(spatial.detach_for_destruction(&catalog,id,&mut runtime.base,&runtime.definition,pc).is_err());
            spatial.bindings[0].source_precision = original_pc;
            assert_eq!(snapshot(&spatial.index), before);
            spatial.detach_for_destruction(&catalog,id,&mut runtime.base,&runtime.definition,pc).unwrap();
            flags = runtime.base.thing.flags;
            assert_eq!(flags & (1<<26),0);
            assert_eq!(std::ptr::from_ref(&runtime.base),address);
            assert_eq!(runtime.base.actor.as_ref().unwrap().components.iter()
                .map(|(s,c)|(s,std::ptr::from_ref(c))).collect::<Vec<_>>(),components);
        }
        assert!(attempt.registry.get(id).is_some(),"globalGTID retirement is a later caller action");
        assert!(spatial.key(id).is_err());
        assert!(spatial.index.member(key).is_err());
        assert!(spatial.index.resolve_reference(id.raw()).is_none());
        let expected = native["rows"].as_array().unwrap().iter().find(|r|r["precision"]==json!(pc)
            &&r["native_class"]==4&&r["member_present"]==true).unwrap();
        assert_eq!(json!({"tiles":snapshot(&spatial.index),"member_flags":{key.to_string():flags},"trace":spatial.index.take_events()}),expected["expected"]["after"]);

        //A second ACTUAL Kingdom is bound for a query, but never inserted.
        //Native4EE560 then skips both descriptor getters and membership writes.
        let second=attempt.registry.iter().find(|e|e.id!=id&&matches!(e.value,RestoredRecord::Kingdom(_))).unwrap().id;
        let mut held=std::mem::replace(attempt.registry.value_mut(second).unwrap(),RestoredRecord::Loading);
        let RestoredRecord::Kingdom(kingdom)=&mut held else{unreachable!()};
        let runtime=kingdom.runtime.as_mut().unwrap();
        assert_eq!(runtime.base.thing.flags&(1<<26),0);
        spatial.prepare(&catalog,&Record{owner:second,shell:&mut runtime.base,definition:&runtime.definition,registry:&mut attempt.registry}).unwrap();
        *attempt.registry.value_mut(second).unwrap()=held;
        {
            let RestoredRecord::Kingdom(kingdom)=attempt.registry.value_mut(second).unwrap()else{unreachable!()};
            let runtime=kingdom.runtime.as_mut().unwrap();
            let flags=runtime.base.thing.flags;
            spatial.detach_for_destruction(&catalog,second,&mut runtime.base,&runtime.definition,pc).unwrap();
            assert_eq!(runtime.base.thing.flags,flags);
        }
        assert!(spatial.bindings.is_empty());
        assert!(spatial.index.take_events().is_empty());
        assert!(attempt.registry.get(second).is_some());
        eprintln!("actual registered Kingdom teardown pc{pc:?}: member removed, query-only detached, both globalIDs and shell/components retained");
    }
}
'''
out = "use super::*;\nuse serde_json::{Value,json};\n\n"
snapshot = old[old.index("fn snapshot("):old.index("#[test]")]
out += snapshot
out += '#[test]\n#[ignore="requires actual source/capture03; bounded registered-shell teardown"]\nfn actual_registered_kingdom_spatial_detach_precedes_global_identity_retirement() {\n'
out += setup
out += '    let native:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-load-spatial-teardown/native-teardown.json")).unwrap();\n'
out += loop + tail
(ROOT / "crates/aa-runtime/src/load_spatial_teardown_tests.rs").write_text(out)

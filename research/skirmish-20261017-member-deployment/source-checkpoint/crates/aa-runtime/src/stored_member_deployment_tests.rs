use super::*;
#[test]
#[ignore = "requires actual original/v14 source and completed source Bank"]
fn actual_canonical_deployment_fields_and_completed_recruiting_lists_use_same_bank() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let roots = ["Thing", "Unit", "Projectile", "VFX"]
        .into_iter()
        .flat_map(|tag| actors.resolved_definitions(tag).unwrap())
        .collect::<Vec<_>>();
    let mut selected = [None, None];
    for actor in &actors.actors {
        if actors.native_category(&actor.id).unwrap().native_kind != 2 {
            continue;
        }
        let Some(node) = roots
            .iter()
            .find(|n| n.sources().first() == Some(&actor.source))
        else {
            continue;
        };
        let info = node
            .child("DenizenComponent")
            .unwrap()
            .map(|n| n.child("DeploymentInfo").unwrap())
            .flatten();
        let index = usize::from(info.is_some());
        if selected[index].is_none() {
            selected[index] = Some(catalog.actor_bindings(&actor.id, pc).unwrap());
        }
        if selected.iter().all(Option::is_some) {
            break;
        }
    }
    assert!(
        selected.iter().all(Option::is_some),
        "actual absent and positive DeploymentInfo sources required"
    );
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut output = vec![];
    for (index, binding) in selected.into_iter().enumerate() {
        let binding = binding.unwrap();
        let d = &binding.definition;
        let fields = SourceFields::bind(&catalog, &bank, d).unwrap();
        assert_eq!(fields.enabled_3ec(), index == 1);
        assert_eq!(fields.source_precision(), pc);
        fields.require_in_bank(&catalog, &bank).unwrap();
        assert!(fields.require_in_bank(&catalog, &Bank::default()).is_err());
        let words = [0x3dc, 0x3e0, 0x3e4, 0x3e8].map(|offset| fields.word(offset).unwrap());
        if index == 0 {
            assert_eq!(words, [0; 4]);
        } else {
            let info = node(&catalog, d)
                .unwrap()
                .child("DenizenComponent")
                .unwrap()
                .unwrap()
                .child("DeploymentInfo")
                .unwrap()
                .unwrap();
            let creation = point(&info, "creation_offset", pc).unwrap();
            let deployment = point(&info, "deployment_offset", pc).unwrap();
            assert_eq!(
                words,
                [creation[0], creation[1], deployment[0], deployment[1]]
            );
        }
        let references = RecruitingStructures::bind(&catalog, &bank, d).unwrap();
        references.require_in_bank(&catalog, &bank).unwrap();
        assert!(
            references
                .require_in_bank(&catalog, &Bank::default())
                .is_err()
        );
        let actual = catalog.native_recruit_backrefs(pc).unwrap();
        assert_eq!(
            references.ids(),
            actual.definition(&d.proof.id).unwrap().ids
        );
        assert!(Arc::ptr_eq(&references.owner, &actual));
        output.push(serde_json::json!({"id":d.proof.id,"source":d.proof.source,"precision":pc,"enabled":fields.enabled_3ec(),"words":words,"recruiting_structure_ids":references.ids()}));
    }
    println!(
        "{}",
        serde_json::json!({"scope":"immutable actual source projection and same completed17C owner; no initialized World or deployment child success","rows":output})
    );
}

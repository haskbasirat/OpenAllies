use super::*;
use crate::{
    resource_objects::Entry,
    texture_resource_load::{Backend as TextureBackend, Diagnostic, Variable},
};
///The Resource fields come from the separately proved native default factory.
///This checks concrete SAME-owner wiring; it is not a complete native depot,
///thread, renderer initialization or mission execution claim.
#[test]
#[ignore = "requires actual source backend; bounded constructed Resource field fixture only"]
fn concrete_default_loader_same_entry_and_required_prefix_all_pc() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let files = aa_rules::sources::V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let mut app = Application::from_sources(files);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-view-asset-backend/constructed-resource-field-app");
    std::fs::create_dir_all(&path).unwrap();
    app.configure_portable_work_depot(&path).unwrap();
    struct Fields {
        fail: bool,
        reads: Vec<Variable>,
        events: Vec<Diagnostic>,
        renderers: usize,
    }
    impl TextureBackend for Fields {
        fn variable_current(&mut self, variable: Variable) -> Result<u8> {
            self.reads.push(variable);
            if self.fail {
                Err("actual-current fixture unavailable".into())
            } else {
                Ok(0)
            }
        }
        fn diagnostic(
            &mut self,
            _: &crate::resource_objects::Handle,
            event: Diagnostic,
        ) -> Result<()> {
            self.events.push(event);
            Ok(())
        }
        //An explicit input of the native renderer-gate fixture. This is not the
        //modern application producer; that path requires the real sealed device.
        fn renderer_b442c0_present(&mut self) -> Result<bool> {
            self.renderers += 1;
            Ok(false)
        }
    }
    for (pc_index, pc) in [
        aa_numeric::Precision::Single24,
        aa_numeric::Precision::Double53,
        aa_numeric::Precision::Extended64,
    ]
    .into_iter()
    .enumerate()
    {
        for fail in [false, true] {
            let ids = format!("default-field-{pc_index}-{fail}.tga")
                .encode_utf16()
                .collect::<Vec<_>>();
            let key = resource_map_cache::normalized_key(&ids).unwrap();
            let resource = {
                let mut manager = app.manager.lock().unwrap();
                let id = manager.work.as_ref().unwrap().id.clone();
                let entry =
                    Entry::default_texture(&app.origin, ids, &app.resource_priority, id).unwrap();
                let handle = entry.handle.clone();
                manager.resources.insert(key, entry);
                handle
            };
            let mut backend = Fields {
                fail,
                reads: vec![],
                events: vec![],
                renderers: 0,
            };
            let mut resources = app.resources();
            let result = resources.load_texture_4c(&resource, pc, &mut backend);
            assert_eq!(backend.reads, vec![Variable::ResourceNoNiTextures]);
            if fail {
                assert_eq!(result.err().unwrap(), "actual-current fixture unavailable");
                assert_eq!(
                    resources.texture_load_stage(&resource).unwrap(),
                    Some(crate::texture_load_kernel::Stage::Policy)
                );
                assert!(resources.texture_5a63d0(&resource).unwrap().is_none());
                assert_eq!(resources.memory_words_0c_10(&resource).unwrap(), [0, 0]);
                assert!(backend.events.is_empty());
            } else {
                assert!(result.unwrap());
                assert_eq!(backend.events, vec![Diagnostic::Loading]);
                assert_eq!(backend.renderers, 1);
                let source = resources.texture_5a63d0(&resource).unwrap().unwrap();
                assert_eq!(resources.source_reference_count(&source).unwrap(), 1);
                let pixels = resources
                    .source_pixels_for_renderer(&source)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    [
                        pixels.pixels().levels()[0].width(),
                        pixels.pixels().levels()[0].height()
                    ],
                    [2, 2]
                );
                assert_eq!(
                    resources.memory_words_0c_10(&resource).unwrap(),
                    [0x128, 0x128]
                );
                assert_eq!(
                    resources.texture_load_stage(&resource).unwrap(),
                    Some(crate::texture_load_kernel::Stage::Complete)
                );
                //Texture vf4C never sets these caller-owned load/bounds bits.
                assert_eq!(resources.flags_08(&resource).unwrap(), 0x200040);
            }
            assert!(
                resources
                    .load_texture_4c(&resource, pc, &mut backend)
                    .err()
                    .unwrap()
                    .contains("retains a prior native load call")
            );
        }
    }
}

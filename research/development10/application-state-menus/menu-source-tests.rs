#[path="../../../crates/aa-client/src/battle_menus.rs"]mod battle_menus;
#[test]fn actual_three_interfaces_remain_owned_across_switches(){
 let base=std::path::Path::new("E:/Games/Axis and Alies/reconstruction");
 let data=aa_ui::fs::DataRoot::with_overlay(base.join("assets/original/Data"),base.join("native-game/runtime-expanded-v14/Data"));
 let mut ui=aa_ui::ui::Ui::new(data,aa_ui::layout::Screen{width:1024,height:768});
 let mut menus=battle_menus::State::new();let handles=menus.bind(&mut ui).unwrap();
 for h in [&handles.main,&handles.splash,&handles.loading,&handles.main,&handles.loading]{menus.show(&mut ui,h).unwrap();assert!(!ui.root.children.is_empty());}
 let mut other=battle_menus::State::new();other.bind(&mut ui).unwrap();assert!(other.show(&mut ui,&handles.main).is_err());
}

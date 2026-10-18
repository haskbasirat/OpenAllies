// Validate the exact production WGSL with the already-built wgpu Naga parser.
#[test]
fn native_scene_uniform_layout_matches_all_three_shader_pipelines(){
    for (name,source)in [
        ("scene",include_str!("../../crates/aa-client/src/scene.wgsl")),
        ("dynamic",include_str!("../../crates/aa-client/src/dynamic.wgsl")),
        ("particles",include_str!("../../crates/aa-client/src/particles.wgsl")),
    ]{
        let module=naga::front::wgsl::parse_str(source).unwrap_or_else(|e|panic!("{name}: {}",e.emit_to_string(source)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
        let camera=module.types.iter().find_map(|(_,ty)|(ty.name.as_deref()==Some("Camera")).then_some(&ty.inner)).unwrap();
        let naga::TypeInner::Struct{span,members}=camera else{panic!("Camera is not a struct")};
        assert_eq!(*span,368,"{name}");assert_eq!(members.len(),11,"{name}");
        for (field,offset)in [("model_root",128),("root_inverse",192),("root_normal",256),("light_ambient",320),("light_diffuse",336),("light_specular",352)]{
            assert_eq!(members.iter().find(|m|m.name.as_deref()==Some(field)).unwrap().offset,offset,"{name}/{field}");
        }
    }
}

//! The native NiRTTI hierarchy: every (class, base class) pair constructed by
//! AA.exe's C++ static initializers (push base; push name; mov ecx, rtti;
//! call 872E40), with the RTTI object address. `NiObject` and `NiShader`
//! have no base.

pub const RTTI: [(&str, Option<&str>); 183] = [
    ("KKC_Camera", Some("NiCamera")), // 0xb40040
    ("KKC_CustomEffectExtraData", Some("NiExtraData")), // 0xb40688
    ("KKC_ExtendShader", Some("NiD3DDefaultShader")), // 0xb406f0
    ("KKC_OverrideShader", Some("NiD3DShader")), // 0xb406e8
    ("KKC_RScreenGeometryManager", Some("NiAVObject")), // 0xb40878
    ("KKC_StencilShadowVertexShader", Some("KKC_VertexShader")), // 0xb40744
    ("KKC_VertexShader", Some("KKC_OverrideShader")), // 0xb406f8
    ("NiAVObject", Some("NiObjectNET")), // 0xb43e10
    ("NiAccumulator", Some("NiObject")), // 0xb46148
    ("NiAlphaAccumulator", Some("NiBackToFrontAccumulator")), // 0xb44488
    ("NiAlphaController", Some("NiFloatController")), // 0xb45f04
    ("NiAlphaProperty", Some("NiProperty")), // 0xb43f34
    ("NiAmbientLight", Some("NiLight")), // 0xb48b10
    ("NiBSPNode", Some("NiNode")), // 0xb48ad0
    ("NiBackToFrontAccumulator", Some("NiAccumulator")), // 0xb46140
    ("NiBillboardNode", Some("NiNode")), // 0xb446c0
    ("NiBinaryExtraData", Some("NiExtraData")), // 0xb45f74
    ("NiBltSource", Some("NiObject")), // 0xb48ae8
    ("NiBoneLODController", Some("NiTimeController")), // 0xb45efc
    ("NiBooleanExtraData", Some("NiExtraData")), // 0xb48ad8
    ("NiCamera", Some("NiAVObject")), // 0xb43e74
    ("NiCollisionData", Some("NiCollisionObject")), // 0xb48b1c
    ("NiCollisionObject", Some("NiObject")), // 0xb45f64
    ("NiColorData", Some("NiObject")), // 0xb45ef4
    ("NiColorExtraData", Some("NiExtraData")), // 0xb48a20
    ("NiColorExtraDataController", Some("NiExtraDataController")), // 0xb45e48
    ("NiControllerManager", Some("NiTimeController")), // 0xb44604
    ("NiControllerSequence", Some("NiObject")), // 0xb4419c
    ("NiD3DController", Some("NiTimeController")), // 0xb46118
    ("NiD3DDefaultShader", Some("NiD3DShader")), // 0xb446c8
    ("NiD3DShader", Some("NiD3DShaderInterface")), // 0xb446b0
    ("NiD3DShaderInterface", Some("NiShader")), // 0xb446b8
    ("NiDX9Renderer", Some("NiRenderer")), // 0xb44408
    ("NiDirectionalLight", Some("NiLight")), // 0xb442dc
    ("NiDitherProperty", Some("NiProperty")), // 0xb442f0
    ("NiDynamicEffect", Some("NiAVObject")), // 0xb4460c
    ("NiExtraData", Some("NiObject")), // 0xb43f0c
    ("NiExtraDataController", Some("NiTimeController")), // 0xb48a18
    ("NiFlipController", Some("NiTimeController")), // 0xb43f2c
    ("NiFloatController", Some("NiTimeController")), // 0xb48a30
    ("NiFloatData", Some("NiObject")), // 0xb45eec
    ("NiFloatExtraData", Some("NiExtraData")), // 0xb48ac0
    ("NiFloatExtraDataController", Some("NiExtraDataController")), // 0xb45e58
    ("NiFloatsExtraData", Some("NiExtraData")), // 0xb48a28
    ("NiFloatsExtraDataController", Some("NiExtraDataController")), // 0xb45e50
    ("NiFogProperty", Some("NiProperty")), // 0xb442fc
    ("NiGeomMorpherController", Some("NiMorpherController")), // 0xb45ee4
    ("NiGeometry", Some("NiAVObject")), // 0xb43cf0
    ("NiGeometryData", Some("NiObject")), // 0xb4473c
    ("NiGravity", Some("NiParticleModifier")), // 0xb46190
    ("NiGyroscopeNode", Some("NiNode")), // 0xb3a1d4
    ("NiIntegerExtraData", Some("NiExtraData")), // 0xb43f04
    ("NiIntegersExtraData", Some("NiExtraData")), // 0xb48ab8
    ("NiKeyframeController", Some("NiTimeController")), // 0xb45edc
    ("NiKeyframeData", Some("NiObject")), // 0xb45ed4
    ("NiKeyframeManager", Some("NiTimeController")), // 0xb45ecc
    ("NiLODData", Some("NiObject")), // 0xb49ebc
    ("NiLODNode", Some("NiSwitchNode")), // 0xb48ab0
    ("NiLight", Some("NiDynamicEffect")), // 0xb445fc
    ("NiLightColorController", Some("NiTimeController")), // 0xb45ec0
    ("NiLines", Some("NiGeometry")), // 0xb4441c
    ("NiLinesData", Some("NiGeometryData")), // 0xb46138
    ("NiLookAtController", Some("NiTimeController")), // 0xb45eb8
    ("NiMaterialColorController", Some("NiTimeController")), // 0xb45ea8
    ("NiMaterialProperty", Some("NiProperty")), // 0xb43f40
    ("NiMeshPSysData", Some("NiPSysData")), // 0xb46268
    ("NiMeshParticleSystem", Some("NiParticleSystem")), // 0xb46270
    ("NiMorphData", Some("NiObject")), // 0xb45eb0
    ("NiMorpherController", Some("NiTimeController")), // 0xb44428
    ("NiNode", Some("NiAVObject")), // 0xb43cd8
    ("NiObject", None), // 0xb43dd8
    ("NiObjectNET", Some("NiObject")), // 0xb43de4
    ("NiPSysAgeDeathModifier", Some("NiPSysModifier")), // 0xb46260
    ("NiPSysBombModifier", Some("NiPSysModifier")), // 0xb45fe0
    ("NiPSysBoundUpdateModifier", Some("NiPSysModifier")), // 0xb46258
    ("NiPSysBoxEmitter", Some("NiPSysVolumeEmitter")), // 0xb46250
    ("NiPSysCollider", Some("NiObject")), // 0xb48b2c
    ("NiPSysColliderManager", Some("NiPSysModifier")), // 0xb45fd8
    ("NiPSysColorModifier", Some("NiPSysModifier")), // 0xb46248
    ("NiPSysCylinderEmitter", Some("NiPSysVolumeEmitter")), // 0xb46240
    ("NiPSysData", Some("NiParticlesData")), // 0xb46238
    ("NiPSysDragFieldModifier", Some("NiPSysModifier")), // 0xb46230
    ("NiPSysDragModifier", Some("NiPSysModifier")), // 0xb46228
    ("NiPSysEmitter", Some("NiPSysModifier")), // 0xb46280
    ("NiPSysEmitterCtlr", Some("NiPSysModifierCtlr")), // 0xb44614
    ("NiPSysEmitterCtlrData", Some("NiObject")), // 0xb46220
    ("NiPSysEmitterDeclinationCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb46218
    ("NiPSysEmitterInitialRadiusCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb46210
    ("NiPSysEmitterLifeSpanCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb46208
    ("NiPSysEmitterPlanarAngleCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb46200
    ("NiPSysEmitterSpeedCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb461f8
    ("NiPSysFieldModifier", Some("NiPSysModifier")), // 0xb48b5c
    ("NiPSysGravityModifier", Some("NiPSysModifier")), // 0xb45fd0
    ("NiPSysGravityStrengthCtlr", Some("NiPSysModifierFloatCtlr")), // 0xb461f0
    ("NiPSysGrowFadeModifier", Some("NiPSysModifier")), // 0xb461e8
    ("NiPSysMeshEmitter", Some("NiPSysEmitter")), // 0xb45fc8
    ("NiPSysMeshUpdateModifier", Some("NiPSysModifier")), // 0xb461e0
    ("NiPSysModifier", Some("NiObject")), // 0xb48b24
    ("NiPSysModifierActiveCtlr", Some("NiPSysModifierBoolCtlr")), // 0xb461d8
    ("NiPSysModifierBoolCtlr", Some("NiPSysModifierCtlr")), // 0xb48b64
    ("NiPSysModifierCtlr", Some("NiTimeController")), // 0xb46278
    ("NiPSysModifierFloatCtlr", Some("NiPSysModifierCtlr")), // 0xb48b6c
    ("NiPSysPlanarCollider", Some("NiPSysCollider")), // 0xb45fc0
    ("NiPSysPositionModifier", Some("NiPSysModifier")), // 0xb461d0
    ("NiPSysResetOnLoopCtlr", Some("NiTimeController")), // 0xb461c8
    ("NiPSysRotationModifier", Some("NiPSysModifier")), // 0xb461c0
    ("NiPSysSpawnModifier", Some("NiPSysModifier")), // 0xb461b0
    ("NiPSysSphereEmitter", Some("NiPSysVolumeEmitter")), // 0xb461a8
    ("NiPSysSphericalCollider", Some("NiPSysCollider")), // 0xb45fb8
    ("NiPSysTurbulenceFieldModifier", Some("NiPSysModifier")), // 0xb461a0
    ("NiPSysUpdateCtlr", Some("NiTimeController")), // 0xb46198
    ("NiPSysVolumeEmitter", Some("NiPSysEmitter")), // 0xb45fb0
    ("NiPalette", Some("NiObject")), // 0xb45f84
    ("NiParticleBomb", Some("NiParticleModifier")), // 0xb46188
    ("NiParticleCollider", Some("NiParticleModifier")), // 0xb48b4c
    ("NiParticleColorModifier", Some("NiParticleModifier")), // 0xb46180
    ("NiParticleGrowFade", Some("NiParticleModifier")), // 0xb46178
    ("NiParticleMeshModifier", Some("NiParticleModifier")), // 0xb46170
    ("NiParticleMeshes", Some("NiParticles")), // 0xb48aa8
    ("NiParticleMeshesData", Some("NiParticlesData")), // 0xb48aa0
    ("NiParticleModifier", Some("NiObject")), // 0xb48b54
    ("NiParticleRotation", Some("NiParticleModifier")), // 0xb46168
    ("NiParticleSystem", Some("NiParticles")), // 0xb4461c
    ("NiParticleSystemController", Some("NiTimeController")), // 0xb45fe8
    ("NiParticles", Some("NiGeometry")), // 0xb43ce0
    ("NiParticlesData", Some("NiGeometryData")), // 0xb45f44
    ("NiPathController", Some("NiTimeController")), // 0xb45ea0
    ("NiPixelData", Some("NiObject")), // 0xb43e7c
    ("NiPlanarCollider", Some("NiParticleCollider")), // 0xb46158
    ("NiPointLight", Some("NiLight")), // 0xb445f4
    ("NiPosData", Some("NiObject")), // 0xb45e90
    ("NiProperty", Some("NiObjectNET")), // 0xb43dcc
    ("NiRangeLODData", Some("NiLODData")), // 0xb48a98
    ("NiRenderedTexture", Some("NiTexture")), // 0xb44414
    ("NiRenderer", Some("NiObject")), // 0xb442c4
    ("NiRendererSpecificProperty", Some("NiProperty")), // 0xb4432c
    ("NiRollController", Some("NiFloatController")), // 0xb45e88
    ("NiScreenGeometry", Some("NiTriShape")), // 0xb48a88
    ("NiScreenGeometryData", Some("NiTriShapeData")), // 0xb48a50
    ("NiScreenLODData", Some("NiLODData")), // 0xb48a90
    ("NiScreenPolygon", Some("NiObject")), // 0xb442cc
    ("NiScreenSpaceCamera", Some("NiCamera")), // 0xb48ac8
    ("NiScreenTexture", Some("NiObject")), // 0xb442d4
    ("NiSequence", Some("NiObject")), // 0xb441ac
    ("NiSequenceStreamHelper", Some("NiObjectNET")), // 0xb45e80
    ("NiShadeProperty", Some("NiProperty")), // 0xb44308
    ("NiShader", None), // 0xb44718
    ("NiSkinData", Some("NiObject")), // 0xb48a38
    ("NiSkinInstance", Some("NiObject")), // 0xb45f4c
    ("NiSkinPartition", Some("NiObject")), // 0xb43f4c
    ("NiSortAdjustNode", Some("NiNode")), // 0xb48a80
    ("NiSourceCubeMap", Some("NiSourceTexture")), // 0xb48a78
    ("NiSourceTexture", Some("NiTexture")), // 0xb43f24
    ("NiSpecularProperty", Some("NiProperty")), // 0xb44314
    ("NiSphericalCollider", Some("NiParticleCollider")), // 0xb46150
    ("NiSpotLight", Some("NiPointLight")), // 0xb48a70
    ("NiStencilProperty", Some("NiProperty")), // 0xb442e4
    ("NiStringExtraData", Some("NiExtraData")), // 0xb45f6c
    ("NiStringsExtraData", Some("NiExtraData")), // 0xb441b4
    ("NiSwitchNode", Some("NiNode")), // 0xb48a68
    ("NiSwitchStringExtraData", Some("NiExtraData")), // 0xb48a60
    ("NiTextKeyExtraData", Some("NiExtraData")), // 0xb45e78
    ("NiTexture", Some("NiObjectNET")), // 0xb44444
    ("NiTextureEffect", Some("NiDynamicEffect")), // 0xb43dec
    ("NiTextureTransformController", Some("NiTimeController")), // 0xb45e40
    ("NiTexturingProperty", Some("NiProperty")), // 0xb43df4
    ("NiTimeController", Some("NiObject")), // 0xb44628
    ("NiTriBasedGeom", Some("NiGeometry")), // 0xb43ce8
    ("NiTriBasedGeomData", Some("NiGeometryData")), // 0xb43f54
    ("NiTriShape", Some("NiTriBasedGeom")), // 0xb43f5c
    ("NiTriShapeData", Some("NiTriBasedGeomData")), // 0xb460f4
    ("NiTriShapeDynamicData", Some("NiTriShapeData")), // 0xb44640
    ("NiTriStrips", Some("NiTriBasedGeom")), // 0xb4437c
    ("NiTriStripsData", Some("NiTriBasedGeomData")), // 0xb46108
    ("NiUVController", Some("NiTimeController")), // 0xb44430
    ("NiUVData", Some("NiObject")), // 0xb45e70
    ("NiVectorExtraData", Some("NiExtraData")), // 0xb48a58
    ("NiVertWeightsExtraData", Some("NiExtraData")), // 0xb45f5c
    ("NiVertexColorProperty", Some("NiProperty")), // 0xb43f14
    ("NiVisController", Some("NiTimeController")), // 0xb45e68
    ("NiVisData", Some("NiObject")), // 0xb45e60
    ("NiWireframeProperty", Some("NiProperty")), // 0xb43e00
    ("NiZBufferProperty", Some("NiProperty")), // 0xb44320
];

/// `NiIsKindOf`: walks the native base-class chain.
pub fn is_kind_of(class: &str, base: &str) -> bool {
    let mut current = Some(class);
    while let Some(name) = current {
        if name == base {
            return true;
        }
        current = RTTI.iter().find(|(n, _)| *n == name).and_then(|(_, parent)| *parent);
    }
    false
}

/// The base class of `class`, if it is a native RTTI class.
pub fn base_of(class: &str) -> Option<&'static str> {
    RTTI.iter().find(|(n, _)| *n == class).and_then(|(_, parent)| *parent)
}

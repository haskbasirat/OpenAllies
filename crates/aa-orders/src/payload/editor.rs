//! Scenario editor (154..=297) order payload schemas, drafted from the original-reader traces
//! (research/phase8-orders/gen_schemas.py) and reviewed against the native
//! decoders named in each doc comment. Wire order is the native read order.
#![allow(clippy::unnecessary_wraps)]
use super::{Registry, Ser, ThingCheck};
use crate::Result;

pub(super) fn schema(descriptor: usize) -> Option<super::Schema> {
    Some(match descriptor {
        154 => p154,
        155 => p154,
        156 => p156,
        157 => p156,
        158 => p156,
        159 => p154,
        160 => p154,
        161 => p156,
        162 => p156,
        163 => p156,
        164 => p164,
        165 => p164,
        166 => p166,
        167 => p166,
        168 => p166,
        169 => p166,
        170 => p166,
        171 => p166,
        172 => p166,
        173 => p166,
        174 => p174,
        175 => p175,
        176 => p176,
        178 => p166,
        179 => p166,
        180 => p166,
        181 => p181,
        182 => p166,
        183 => p183,
        184 => p184,
        185 => p166,
        186 => p166,
        187 => p166,
        188 => p166,
        189 => p166,
        190 => p166,
        191 => p166,
        192 => p166,
        193 => p174,
        194 => p175,
        195 => p176,
        197 => p166,
        198 => p166,
        199 => p166,
        200 => p181,
        201 => p166,
        202 => p183,
        203 => p184,
        204 => p166,
        205 => p166,
        206 => p206,
        207 => p207,
        208 => p208,
        209 => p208,
        210 => p166,
        211 => p211,
        212 => p212,
        213 => p206,
        214 => p207,
        215 => p208,
        216 => p208,
        217 => p166,
        218 => p211,
        219 => p212,
        220 => p220,
        221 => p166,
        222 => p222,
        223 => p223,
        224 => p224,
        225 => p225,
        226 => p226,
        227 => p226,
        228 => p228,
        229 => p229,
        230 => p230,
        231 => p231,
        232 => p231,
        233 => p233,
        234 => p234,
        235 => p234,
        236 => p220,
        237 => p166,
        238 => p222,
        239 => p223,
        240 => p224,
        241 => p225,
        242 => p226,
        243 => p226,
        244 => p228,
        245 => p229,
        246 => p230,
        247 => p231,
        248 => p231,
        249 => p233,
        250 => p234,
        251 => p234,
        252 => p252,
        253 => p253,
        254 => p252,
        255 => p253,
        256 => p256,
        257 => p256,
        258 => p166,
        259 => p259,
        260 => p252,
        261 => p259,
        262 => p166,
        263 => p259,
        264 => p252,
        265 => p259,
        266 => p266,
        267 => p267,
        268 => p266,
        269 => p267,
        270 => p166,
        271 => p259,
        272 => p252,
        273 => p252,
        274 => p252,
        275 => p252,
        276 => p252,
        277 => p166,
        278 => p259,
        279 => p252,
        280 => p252,
        281 => p252,
        282 => p252,
        283 => p252,
        284 => p284,
        285 => p284,
        286 => p286,
        287 => p287,
        288 => p286,
        289 => p287,
        290 => p166,
        291 => p166,
        292 => p166,
        294 => p174,
        295 => p166,
        297 => p174,
        _ => return None,
    })
}

/// 154 KKC_EditorChangeWorldRuleLockedRequest (handler 0x6a3330, process 0x6b6120)
/// 155 KKC_EditorChangeWorldRuleBoolRequest (handler 0x6a35d0, process 0x6b6140)
/// 159 KKC_EditorChangeWorldRuleLockedDecree (handler 0x6a1a60, process 0x6b6120)
/// 160 KKC_EditorChangeWorldRuleBoolDecree (handler 0x6a1b50, process 0x6b6140)
/// native reads: 0x411FA0, 0x6A18F0
pub(super) fn p154(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa88))?;
    s.uint(1)?;
    Ok(())
}

/// 156 KKC_EditorChangeWorldRuleIntRequest (handler 0x6a3870, process 0x6b6160)
/// 157 KKC_EditorChangeWorldRuleFixedRequest (handler 0x6a3b10, process 0x6b6180)
/// 158 KKC_EditorChangeWorldRuleEnumRequest (handler 0x6a3db0, process 0x6b61a0)
/// 161 KKC_EditorChangeWorldRuleIntDecree (handler 0x6a1c40, process 0x6b6160)
/// 162 KKC_EditorChangeWorldRuleFixedDecree (handler 0x6a1d30, process 0x6b6180)
/// 163 KKC_EditorChangeWorldRuleEnumDecree (handler 0x6a1e20, process 0x6b61a0)
/// native reads: 0x411FA0, 0x6A1970, 0x6A19F0
pub(super) fn p156(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa88))?;
    s.uint(32)?;
    Ok(())
}

/// 164 KKC_EditorActorRequest (handler 0x6a5410, process dword ptr [edx + 4])
/// 165 KKC_EditorActorDecree (handler 0x6a44b0, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x6A4440, 0x76A600
pub(super) fn p164(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::A8)?;
    s.text()?;
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 166 TGC_EditorSaveOrderRequest (handler 0x6a7610, process 0x75d8e0)
/// 167 TGC_EditorSaveOrderDecree (handler 0x6a7360, process 0x75d8e0)
/// 168 KKC_EditorAddTerrainNoiseRequest (handler 0x6af0e0, process dword ptr [eax + 4])
/// 169 KKC_EditorBeautifyMapRequest (handler 0x6af360, process dword ptr [eax + 4])
/// 170 KKC_EditorClearObjectsRequest (handler 0x6af5e0, process dword ptr [eax + 4])
/// 171 KKC_EditorClearActorsRequest (handler 0x6af860, process dword ptr [eax + 4])
/// 172 KKC_EditorClearMapRequest (handler 0x6afae0, process dword ptr [eax + 4])
/// 173 KKC_EditorClearTriggersRequest (handler 0x6afd60, process dword ptr [eax + 4])
/// 178 KKC_EditorUndoOrderRequest (handler 0x6b0b70, process 0x6b5f40)
/// 179 KKC_EditorRedoOrderRequest (handler 0x6b0df0, process 0x6b5f60)
/// 180 KKC_EditorCopyOrderRequest (handler 0x6b1070, process dword ptr [eax + 4])
/// 182 KKC_EditorSelectedDestroyOrderRequest (handler 0x6b1590, process dword ptr [eax + 4])
/// 185 TGC_EditorPrepareMapOrderRequest (handler 0x6ae8c0, process 0x75d910)
/// 186 TGC_EditorVerifyMapOrderRequest (handler 0x6aeaa0, process 0x75d920)
/// 187 KKC_EditorAddTerrainNoiseDecree (handler 0x6ab5c0, process dword ptr [eax + 4])
/// 188 KKC_EditorBeautifyMapDecree (handler 0x6ab6a0, process dword ptr [eax + 4])
/// 189 KKC_EditorClearObjectsDecree (handler 0x6ab780, process dword ptr [eax + 4])
/// 190 KKC_EditorClearActorsDecree (handler 0x6ab860, process dword ptr [eax + 4])
/// 191 KKC_EditorClearMapDecree (handler 0x6ab940, process dword ptr [eax + 4])
/// 192 KKC_EditorClearTriggersDecree (handler 0x6aba20, process dword ptr [eax + 4])
/// 197 KKC_EditorUndoOrderDecree (handler 0x6abc20, process 0x6b5f40)
/// 198 KKC_EditorRedoOrderDecree (handler 0x6abd00, process 0x6b5f60)
/// 199 KKC_EditorCopyOrderDecree (handler 0x6abde0, process dword ptr [eax + 4])
/// 201 KKC_EditorSelectedDestroyOrderDecree (handler 0x6abfc0, process dword ptr [eax + 4])
/// 204 TGC_EditorPrepareMapOrderDecree (handler 0x6ac1a0, process 0x75d910)
/// 205 TGC_EditorVerifyMapOrderDecree (handler 0x6ac200, process 0x75d920)
/// 210 KKC_EditorOrientSelectOrganizationElementsOrderRequest (handler 0x6bc0b0, process dword ptr [eax + 4])
/// 217 KKC_EditorOrientSelectOrganizationElementsOrderDecree (handler 0x6ba800, process dword ptr [eax + 4])
/// 221 KKC_EditorCreateGTeamRequest (handler 0x6c1b70, process dword ptr [eax + 4])
/// 237 KKC_EditorCreateGTeamDecree (handler 0x6beca0, process dword ptr [eax + 4])
/// 258 KKC_EditorObjectiveAddRequest (handler 0x6cd230, process 0x6b82c0)
/// 262 KKC_EditorObjectiveAddDecree (handler 0x6cbd90, process 0x6b82c0)
/// 270 KKC_EditorCreateRestrictionGroupRequest (handler 0x6d3310, process dword ptr [eax + 4])
/// 277 KKC_EditorCreateRestrictionGroupDecree (handler 0x6d0c40, process dword ptr [eax + 4])
/// 290 KKC_EditorDoodadAutoPlacementRequest (handler 0x6e24e0, process 0x6b5fd0)
/// 291 KKC_EditorDoodadAutoPlacementDecree (handler 0x6e1550, process 0x6b5fd0)
/// 292 KKC_EditorAddTriggerRequest (handler 0x6e50a0, process 0x6b8520)
/// 295 KKC_EditorAddTriggerDecree (handler 0x6e4580, process 0x6b8520)
/// native reads: none (empty payload)
pub(super) fn p166(_s: &mut dyn Ser) -> Result<()> {
    Ok(())
}

/// 174 KKC_EditorRotateSelectionRequest (handler 0x6affe0, process dword ptr [edx + 4])
/// 193 KKC_EditorRotateSelectionDecree (handler 0x6abb00, process dword ptr [edx + 4])
/// 294 KKC_EditorRemoveTriggerRequest (handler 0x6e55d0, process 0x6b8230)
/// 297 KKC_EditorRemoveTriggerDecree (handler 0x6e47a0, process 0x6b8230)
/// native reads: 0x6ABB00, 0x6AFFE0, 0x6E47A0, 0x6E55D0
pub(super) fn p174(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    Ok(())
}

/// 175 KKC_EditorCreateGKingdomRequest (handler 0x6b0290, process dword ptr [edx + 4])
/// 194 KKC_EditorCreateGKingdomDecree (handler 0x6ac330, process dword ptr [eax + 4])
/// native reads: 0x410B50, 0x6AC330, 0x6B0290
pub(super) fn p175(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    s.text()?;
    Ok(())
}

/// 176 KKC_EditorCreateGAreaRequest (handler 0x6b0580, process dword ptr [eax + 4])
/// 195 KKC_EditorCreateGAreaDecree (handler 0x6aed40, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x6AA2B0, 0x6AEC80
pub(super) fn p176(s: &mut dyn Ser) -> Result<()> {
    s.bounded(4)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.def(Registry(0xb39f58))?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 181 KKC_EditorDestroyOrderRequest (handler 0x6b12f0, process dword ptr [edx + 4])
/// 200 KKC_EditorDestroyOrderDecree (handler 0x6abec0, process dword ptr [edx + 4])
/// native reads: 0x76A4B0
pub(super) fn p181(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::Any)?;
    Ok(())
}

/// 183 KKC_EditorThingOrientRequest (handler 0x6b1810, process 0x6b6060)
/// 202 KKC_EditorThingOrientDecree (handler 0x6ac4c0, process 0x6b6060)
/// native reads: 0x410B50, 0x6AA250
pub(super) fn p183(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 184 KKC_EditorHQDetachRequest (handler 0x6b1b00, process 0x6b6400)
/// 203 KKC_EditorHQDetachDecree (handler 0x6ac0a0, process 0x6b6400)
/// native reads: 0x76A5A0
pub(super) fn p184(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::B4)?;
    Ok(())
}

/// 206 KKC_EditorPaintTerrainRequest (handler 0x6bb550, process dword ptr [eax + 4])
/// 213 KKC_EditorPaintTerrainDecree (handler 0x6bb230, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x6AA2B0, 0x6BAFF0
pub(super) fn p206(s: &mut dyn Ser) -> Result<()> {
    s.bounded(4)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.def(Registry(0xb3aae0))?;
    s.uint(32)?;
    Ok(())
}

/// 207 KKC_EditorPaintHeightmapRequest (handler 0x6bb820, process dword ptr [eax + 4])
/// 214 KKC_EditorPaintHeightmapDecree (handler 0x6bb350, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x6AA2B0, 0x6BB100
pub(super) fn p207(s: &mut dyn Ser) -> Result<()> {
    s.bounded(4)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 208 KKC_EditorPasteOrderRequest (handler 0x6bbaf0, process dword ptr [eax + 4])
/// 209 KKC_EditorDragOrderRequest (handler 0x6bbdd0, process dword ptr [eax + 4])
/// 215 KKC_EditorPasteOrderDecree (handler 0x6ba580, process dword ptr [edx + 4])
/// 216 KKC_EditorDragOrderDecree (handler 0x6ba6c0, process dword ptr [edx + 4])
/// native reads: 0x6BA580, 0x6BA6C0, 0x6BBAF0, 0x6BBDD0
pub(super) fn p208(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 211 KKC_EditorOrientSelectionRequest (handler 0x6bc330, process 0x6b69d0)
/// 218 KKC_EditorOrientSelectionDecree (handler 0x6ba8e0, process 0x6b69d0)
/// native reads: 0x6AA250
pub(super) fn p211(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 212 KKC_EditorHQAttachRequest (handler 0x6bc5f0, process 0x6b63c0)
/// 219 KKC_EditorHQAttachDecree (handler 0x6ba9f0, process 0x6b63c0)
/// native reads: 0x76A5A0
pub(super) fn p212(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::B4)?;
    s.thing(ThingCheck::B4)?;
    Ok(())
}

/// 220 KKC_EditorSetRestrictionGroupRequest (handler 0x6c1830, process dword ptr [edx + 4])
/// 236 KKC_EditorSetRestrictionGroupDecree (handler 0x6bf600, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x6BF600, 0x6C1830
pub(super) fn p220(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    s.uint(1)?;
    Ok(())
}

/// 222 KKC_EditorChangeGKingdomFactionRequest (handler 0x6c1df0, process dword ptr [eax + 4])
/// 238 KKC_EditorChangeGKingdomFactionDecree (handler 0x6bff70, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A600
pub(super) fn p222(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa4c))?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 223 KKC_EditorChangeGKingdomNationRequest (handler 0x6c2090, process dword ptr [eax + 4])
/// 239 KKC_EditorChangeGKingdomNationDecree (handler 0x6c0070, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A600
pub(super) fn p223(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa44))?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 224 KKC_EditorChangeGKingdomResourceStockpileRequest (handler 0x6c2330, process dword ptr [eax + 4])
/// 240 KKC_EditorChangeGKingdomResourceStockpileDecree (handler 0x6c0170, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x6BFD00, 0x76A600
pub(super) fn p224(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3a660))?;
    s.uint(32)?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 225 KKC_EditorChangeGKingdomColorRequest (handler 0x6c25d0, process dword ptr [eax + 4])
/// 241 KKC_EditorChangeGKingdomColorDecree (handler 0x6c0270, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A600
pub(super) fn p225(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa60))?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 226 KKC_EditorAddHeroOrderRequest (handler 0x6c2870, process dword ptr [edx + 4])
/// 227 KKC_EditorRemoveHeroOrderRequest (handler 0x6c2b10, process dword ptr [edx + 4])
/// 242 KKC_EditorAddHeroOrderDecree (handler 0x6bed80, process dword ptr [edx + 4])
/// 243 KKC_EditorRemoveHeroOrderDecree (handler 0x6bee90, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A600
pub(super) fn p226(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb39f58))?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 228 KKC_EditorAddPropertyOrderRequest (handler 0x6c2db0, process dword ptr [eax + 4])
/// 244 KKC_EditorAddPropertyOrderDecree (handler 0x6c0370, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A600
pub(super) fn p228(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3aa2c))?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 229 KKC_EditorKingdomUsesDefaultResourcesOrderRequest (handler 0x6c3050, process dword ptr [eax + 4])
/// 245 KKC_EditorKingdomUsesDefaultResourcesOrderDecree (handler 0x6befa0, process dword ptr [eax + 4])
/// native reads: 0x6BEFA0, 0x6C3050, 0x76A600
pub(super) fn p229(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 230 KKC_EditorChangeGKingdomOpenOrAIRequest (handler 0x6c3320, process dword ptr [eax + 4])
/// 246 KKC_EditorChangeGKingdomOpenOrAIDecree (handler 0x6bf0c0, process dword ptr [edx + 4])
/// native reads: 0x6BCE90, 0x76A600
pub(super) fn p230(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    s.uint(1)?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 231 KKC_EditorChangeGKingdomSAIRequest (handler 0x6c35c0, process dword ptr [edx + 4])
/// 232 KKC_EditorChangeGKingdomNameRequest (handler 0x6c38a0, process dword ptr [edx + 4])
/// 247 KKC_EditorChangeGKingdomSAIDecree (handler 0x6bf7d0, process dword ptr [edx + 4])
/// 248 KKC_EditorChangeGKingdomNameDecree (handler 0x6bf940, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x76A600
pub(super) fn p231(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 233 KKC_EditorChangeGTeamNameRequest (handler 0x6c3b80, process dword ptr [edx + 4])
/// 249 KKC_EditorChangeGTeamNameDecree (handler 0x6bfab0, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x76A670
pub(super) fn p233(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.thing(ThingCheck::Ac)?;
    Ok(())
}

/// 234 KKC_EditorAddGKingdomToGTeamRequest (handler 0x6c3e60, process dword ptr [edx + 4])
/// 235 KKC_EditorRemoveGKingdomFromGTeamRequest (handler 0x6c4100, process dword ptr [edx + 4])
/// 250 KKC_EditorAddGKingdomToGTeamDecree (handler 0x6bf1c0, process dword ptr [edx + 4])
/// 251 KKC_EditorRemoveGKingdomFromGTeamDecree (handler 0x6bf2d0, process dword ptr [edx + 4])
/// native reads: 0x76A600, 0x76A670
pub(super) fn p234(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::A8)?;
    s.thing(ThingCheck::Ac)?;
    Ok(())
}

/// 252 KKC_EditorChangeMapPropertiesRequest (handler 0x6c9d10, process dword ptr [edx + 4])
/// 254 KKC_EditorChangeMapPropertiesDecree (handler 0x6c9230, process dword ptr [edx + 4])
/// 260 KKC_EditorObjectiveChangeDescriptionRequest (handler 0x6cd780, process 0x6b83d0)
/// 264 KKC_EditorObjectiveChangeDescriptionDecree (handler 0x6cc130, process 0x6b83d0)
/// 272 KKC_EditorSetRestrictionGroupNameRequest (handler 0x6d3860, process dword ptr [edx + 4])
/// 273 KKC_EditorCreateTechnologyRestrictionRequest (handler 0x6d3b80, process dword ptr [edx + 4])
/// 274 KKC_EditorCreateActorDataRestrictionRequest (handler 0x6d3ea0, process dword ptr [edx + 4])
/// 275 KKC_EditorRemoveTechnologyRestrictionRequest (handler 0x6d41c0, process dword ptr [edx + 4])
/// 276 KKC_EditorRemoveActorDataRestrictionRequest (handler 0x6d44e0, process dword ptr [edx + 4])
/// 279 KKC_EditorSetRestrictionGroupNameDecree (handler 0x6d0fa0, process dword ptr [edx + 4])
/// 280 KKC_EditorCreateTechnologyRestrictionDecree (handler 0x6d1150, process dword ptr [edx + 4])
/// 281 KKC_EditorCreateActorDataRestrictionDecree (handler 0x6d1300, process dword ptr [edx + 4])
/// 282 KKC_EditorRemoveTechnologyRestrictionDecree (handler 0x6d14b0, process dword ptr [edx + 4])
/// 283 KKC_EditorRemoveActorDataRestrictionDecree (handler 0x6d1660, process dword ptr [edx + 4])
/// native reads: 0x410B50
pub(super) fn p252(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    Ok(())
}

/// 253 KKC_EditorChangeMapTypeRequest (handler 0x6ca030, process dword ptr [edx + 4])
/// 255 KKC_EditorChangeMapTypeDecree (handler 0x6c93e0, process dword ptr [eax + 4])
/// native reads: 0x411FA0
pub(super) fn p253(s: &mut dyn Ser) -> Result<()> {
    s.bounded(7)?;
    Ok(())
}

/// 256 KKC_EditorSetBlankSettingsOrderRequest (handler 0x6cb0d0, process 0x75dc00)
/// 257 KKC_EditorSetBlankSettingsOrderDecree (handler 0x6caaf0, process 0x75dc00)
/// native reads: 0x411FA0, 0x6CA9D0
pub(super) fn p256(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    s.def(Registry(0xb3aa24))?;
    s.def(Registry(0xb3aa34))?;
    s.uint(1)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 259 KKC_EditorObjectiveRemoveRequest (handler 0x6cd4b0, process 0x6b8340)
/// 261 KKC_EditorObjectiveToggleRequiredRequest (handler 0x6cdaa0, process 0x6b60d0)
/// 263 KKC_EditorObjectiveRemoveDecree (handler 0x6cbfd0, process 0x6b8340)
/// 265 KKC_EditorObjectiveToggleRequiredDecree (handler 0x6cc2d0, process 0x6b60d0)
/// 271 KKC_EditorDeleteRestrictionGroupRequest (handler 0x6d3590, process dword ptr [edx + 4])
/// 278 KKC_EditorDeleteRestrictionGroupDecree (handler 0x6d0e40, process dword ptr [edx + 4])
/// native reads: 0x410B50
pub(super) fn p259(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    Ok(())
}

/// 266 KKC_EditorChangeGTeamRelationRequest (handler 0x6cece0, process dword ptr [eax + 4])
/// 268 KKC_EditorChangeGTeamRelationDecree (handler 0x6ce5d0, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A4B0, 0x76A600
pub(super) fn p266(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::Any)?;
    s.thing(ThingCheck::A8)?;
    s.bounded(4)?;
    Ok(())
}

/// 267 KKC_EditorChangeGTeamRelationRequest (handler 0x6cefa0, process dword ptr [eax + 4])
/// 269 KKC_EditorChangeGTeamRelationDecree (handler 0x6ce6f0, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x76A4B0, 0x76A670
pub(super) fn p267(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::Any)?;
    s.thing(ThingCheck::Ac)?;
    s.bounded(4)?;
    Ok(())
}

/// 284 KKC_EditorGAreaPropertiesRequest (handler 0x6dd070, process dword ptr [edx + 4])
/// 285 KKC_EditorGAreaPropertiesDecree (handler 0x6dbae0, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x6DBA60, 0x76A6E0
pub(super) fn p284(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::Bc)?;
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 286 KKC_EditorCreateGCameraRequest (handler 0x6de630, process dword ptr [eax + 4])
/// 288 KKC_EditorCreateGCameraDecree (handler 0x6de2f0, process dword ptr [edx + 4])
/// native reads: 0x411FA0, 0x6AA250, 0x6DE270
pub(super) fn p286(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.def(Registry(0xb39f58))?;
    Ok(())
}

/// 287 KKC_EditorGCameraPropertiesRequest (handler 0x6de900, process dword ptr [edx + 4])
/// 289 KKC_EditorGCameraPropertiesDecree (handler 0x6dde90, process dword ptr [edx + 4])
/// native reads: 0x410B50, 0x6AA250, 0x6DDE90, 0x6DE900, 0x76A750
pub(super) fn p287(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::C0)?;
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

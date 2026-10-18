//! Session/lobby (100..=153, 298..=357) order payload schemas, drafted from the original-reader traces
//! (research/phase8-orders/gen_schemas.py) and reviewed against the native
//! decoders named in each doc comment. Wire order is the native read order.
#![allow(clippy::unnecessary_wraps)]
use super::{NameKind, Registry, Ser, ThingCheck};
use crate::Result;

pub(super) fn schema(descriptor: usize) -> Option<super::Schema> {
    Some(match descriptor {
        100 => p100,
        101 => p101,
        102 => p101,
        103 => p103,
        104 => p100,
        105 => p101,
        106 => p106,
        107 => p100,
        108 => p100,
        109 => p109,
        110 => p106,
        111 => p100,
        112 => p100,
        113 => p109,
        114 => p114,
        115 => p115,
        116 => p106,
        119 => p114,
        120 => p115,
        121 => p106,
        124 => p100,
        125 => p100,
        126 => p126,
        128 => p100,
        129 => p100,
        130 => p126,
        132 => p132,
        133 => p133,
        134 => p133,
        135 => p135,
        136 => p136,
        137 => p137,
        138 => p138,
        139 => p135,
        140 => p140,
        141 => p133,
        142 => p133,
        143 => p132,
        144 => p133,
        145 => p133,
        146 => p135,
        147 => p136,
        148 => p137,
        149 => p138,
        150 => p135,
        151 => p140,
        152 => p133,
        153 => p133,
        298 => p106,
        299 => p114,
        300 => p135,
        301 => p135,
        302 => p114,
        303 => p106,
        304 => p304,
        305 => p305,
        306 => p306,
        307 => p133,
        308 => p308,
        309 => p308,
        310 => p133,
        311 => p114,
        312 => p135,
        313 => p135,
        314 => p114,
        315 => p304,
        316 => p305,
        317 => p306,
        318 => p133,
        319 => p133,
        320 => p106,
        323 => p101,
        324 => p324,
        325 => p325,
        326 => p106,
        327 => p101,
        328 => p324,
        329 => p325,
        330 => p106,
        331 => p101,
        332 => p332,
        333 => p332,
        334 => p101,
        335 => p335,
        336 => p332,
        337 => p109,
        338 => p101,
        339 => p335,
        340 => p332,
        341 => p109,
        342 => p342,
        343 => p343,
        344 => p344,
        345 => p100,
        346 => p346,
        347 => p347,
        348 => p342,
        349 => p343,
        350 => p344,
        351 => p100,
        352 => p346,
        353 => p347,
        354 => p354,
        355 => p354,
        _ => return None,
    })
}

/// 100 TGC_SetTeamManagementTeamsOrderRequest (handler 0x660100, process 0x75e2d0)
/// 104 TGC_SetTeamManagementTeamsOrderDecree (handler 0x65c850, process 0x75e2d0)
/// 107 TGC_SetGameNameOrderRequest (handler 0x672db0, process 0x75d9c0)
/// 108 TGC_SetGamePasswordOrderRequest (handler 0x672fe0, process 0x75d9d0)
/// 111 TGC_SetGameNameOrderDecree (handler 0x671a00, process 0x75d9c0)
/// 112 TGC_SetGamePasswordOrderDecree (handler 0x671ae0, process 0x75d9d0)
/// 124 KKC_HostSetLoadSavedGameOrderRequest (handler 0x68dda0, process 0x75f1c0)
/// 125 KKC_HostSetLoadFilmOrderRequest (handler 0x68dfd0, process 0x75f470)
/// 128 KKC_HostSetLoadSavedGameOrderDecree (handler 0x68aa30, process 0x75f1c0)
/// 129 KKC_HostSetLoadFilmOrderDecree (handler 0x68ab10, process 0x75f470)
/// 345 TGC_ClearKingdomAdditionalActorsOrderRequest (handler 0x9480d0, process 0x75e3a0)
/// 351 TGC_ClearKingdomAdditionalActorsOrderDecree (handler 0x946240, process 0x75e3a0)
/// native reads: 0x410B50
pub(super) fn p100(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    Ok(())
}

/// 101 TGC_StopLaunchCountdownOrderRequest (handler 0x660330, process 0x75dba0)
/// 102 TGC_SetSessionEditorOrderDecree (handler 0x65c650, process 0x75da00)
/// 105 TGC_StopLaunchCountdownOrderDecree (handler 0x65c750, process 0x75dba0)
/// 323 OrderVersionAckRequest (handler 0x766cf0)
/// 327 TGC_NotifyNonHostIsNowHostOrderRequest (handler 0x7732a0, process 0x75dbe0)
/// 331 TGC_NotifyNonHostIsNowHostOrderDecree (handler 0x76dc50, process 0x75dbe0)
/// 334 TGC_MetaGameSaveOrderRequest (handler 0x935dc0, process 0x75d900)
/// 338 TGC_MetaGameSaveOrderDecree (handler 0x934430, process 0x75d900)
/// native reads: none (empty payload)
pub(super) fn p101(_s: &mut dyn Ser) -> Result<()> {
    Ok(())
}

/// 103 TGC_SetPlayerReadyStatusOrderDecree (handler 0x65c6b0, process 0x75db40)
/// native reads: 0x65C6B0, 0x765F70
pub(super) fn p103(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.uint(32)?;
    Ok(())
}

/// 106 TGC_SetMaxPlayersOrderRequest (handler 0x672b80, process 0x75d9b0)
/// 110 TGC_SetMaxPlayersOrderDecree (handler 0x6718e0, process 0x75d9b0)
/// 116 TGC_SetRandomMapMaxPlayersInEditorOrderRequest (handler 0x686450, process 0x75dbc0)
/// 121 TGC_SetRandomMapMaxPlayersInEditorOrderDecree (handler 0x683d70, process 0x75dbc0)
/// 298 TGC_SetPlayerOrderDecree (handler 0x750dd0, process 0x75d710)
/// 303 TGC_SyncFailureOrderDecree (handler 0x750ef0, process 0x75d9a0)
/// 320 TGC_SetPlayerOrderRequest (handler 0x75e9b0, process 0x75d710)
/// 326 TGC_LaunchCountdownProgressOrderRequest (handler 0x773070, process 0x75dbb0)
/// 330 TGC_LaunchCountdownProgressOrderDecree (handler 0x76dbc0, process 0x75dbb0)
/// native reads: 0x6718E0, 0x672B80, 0x683D70, 0x686450, 0x750DD0, 0x750EF0, 0x75E9B0, 0x76DBC0, 0x773070
pub(super) fn p106(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    Ok(())
}

/// 109 TGC_SetAllowObserversOrderRequest (handler 0x673210, process 0x75d9e0)
/// 113 TGC_SetAllowObserversOrderDecree (handler 0x671970, process 0x75d9e0)
/// 337 MGSetFastAITurnOrderRequest (handler 0x9364e0, process 0x9504e0)
/// 341 MGSetFastAITurnOrderDecree (handler 0x9346a0, process 0x9504e0)
/// native reads: 0x671970, 0x673210, 0x9346A0, 0x9364E0
pub(super) fn p109(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    Ok(())
}

/// 114 TGC_SetMapSizeOrderRequest (handler 0x686010, process 0x75d960)
/// 119 TGC_SetMapSizeOrderDecree (handler 0x683c50, process 0x75d960)
/// 299 KKC_SetSimSyncValueOrderRequest (handler 0x7574b0, process 0x75d7f0)
/// 302 TGC_SetSyncValueOrderRequest (handler 0x757b70, process 0x75d980)
/// 311 KKC_SetSimSyncValueOrderDecree (handler 0x750f80, process 0x75d7f0)
/// 314 TGC_SetSyncValueOrderDecree (handler 0x751070, process 0x75d980)
/// native reads: 0x683C50, 0x686010, 0x74D040
pub(super) fn p114(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 115 TGC_SetRMCOrderRequest (handler 0x686250, process 0x75d970)
/// 120 TGC_SetRMCOrderDecree (handler 0x683d00, process 0x75d970)
/// native reads: 0x76A9C0
pub(super) fn p115(s: &mut dyn Ser) -> Result<()> {
    s.named(NameKind::RandomMapConfig)?;
    Ok(())
}

/// 126 TGC_ASyncLoadFileHeadersOrderRequest (handler 0x68cd60, process 0x75e440)
/// 130 TGC_ASyncLoadFileHeadersOrderDecree (handler 0x68abf0, process 0x75e440)
/// native reads: 0x410B50, 0x68ABF0, 0x68CD60
pub(super) fn p126(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    s.uint(1)?;
    Ok(())
}

/// 132 TGC_CreateExplicitAIOrderRequest (handler 0x69a6c0, process 0x75e030)
/// 143 TGC_CreateExplicitAIOrderDecree (handler 0x696e40, process 0x75e030)
/// native reads: 0x410B50
pub(super) fn p132(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    s.text()?;
    Ok(())
}

/// 133 TGC_KickPlayerOrderRequest (handler 0x69a930, process 0x75dae0)
/// 134 TGC_BanPlayerOrderRequest (handler 0x69ab30, process 0x75db10)
/// 141 TGC_RequestAcceptSwapOrderRequest (handler 0x69ba50, process 0x75db60)
/// 142 TGC_RejectCancelSwapOrderRequest (handler 0x69bc50, process 0x75db70)
/// 144 TGC_KickPlayerOrderDecree (handler 0x696760, process 0x75dae0)
/// 145 TGC_BanPlayerOrderDecree (handler 0x6967d0, process 0x75db10)
/// 152 TGC_RequestAcceptSwapOrderDecree (handler 0x6968c0, process 0x75db60)
/// 153 TGC_RejectCancelSwapOrderDecree (handler 0x696930, process 0x75db70)
/// 307 TGC_PostPlayerJoinOrderRequest (handler 0x7583a0, process 0x75da90)
/// 310 TGC_MarkPlayerAuthenticatedOrderRequest (handler 0x758750, process 0x75db80)
/// 318 TGC_PostPlayerJoinOrderDecree (handler 0x751220, process 0x75da90)
/// 319 TGC_MarkPlayerAuthenticatedOrderDecree (handler 0x751290, process 0x75db80)
/// native reads: 0x765F70
pub(super) fn p133(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    Ok(())
}

/// 135 TGC_SetPlayerKingdomOrderRequest (handler 0x69ad30, process 0x75e140)
/// 139 TGC_SetPlayerTeamOrderRequest (handler 0x69b5e0, process 0x75e300)
/// 146 TGC_SetPlayerKingdomOrderDecree (handler 0x696f50, process 0x75e140)
/// 150 TGC_SetPlayerTeamOrderDecree (handler 0x697030, process 0x75e300)
/// 300 TGC_ChallengeOrderRequest (handler 0x757750, process 0x75df10)
/// 301 TGC_ChallengeResponseOrderRequest (handler 0x757960, process 0x75df50)
/// 312 TGC_ChallengeOrderDecree (handler 0x7526b0, process 0x75df10)
/// 313 TGC_ChallengeResponseOrderDecree (handler 0x752790, process 0x75df50)
/// native reads: 0x410B50, 0x765F70
pub(super) fn p135(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.text()?;
    Ok(())
}

/// 136 TGC_SwapPlayerKingdomsOrderRequest (handler 0x69af70, process 0x75db20)
/// 147 TGC_SwapPlayerKingdomsOrderDecree (handler 0x696840, process 0x75db20)
/// native reads: 0x765F70
pub(super) fn p136(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.player()?;
    Ok(())
}

/// 137 TGC_SetKingdomNationOrderRequest (handler 0x69b180, process 0x75e1c0)
/// 148 TGC_SetKingdomNationOrderDecree (handler 0x6975c0, process 0x75e1c0)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p137(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa44))?;
    Ok(())
}

/// 138 TGC_SetKingdomFactionOrderRequest (handler 0x69b3b0, process 0x75e1f0)
/// 149 TGC_SetKingdomFactionOrderDecree (handler 0x6976a0, process 0x75e1f0)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p138(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa4c))?;
    Ok(())
}

/// 140 TGC_SetAIHandicapOrderRequest (handler 0x69b820, process 0x75e340)
/// 151 TGC_SetAIHandicapOrderDecree (handler 0x697780, process 0x75e340)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p140(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa68))?;
    Ok(())
}

/// 304 TGC_UpdateTaskStatusOrderRequest (handler 0x757d40, process 0x75da50)
/// 315 TGC_UpdateTaskStatusOrderDecree (handler 0x752870, process 0x75da50)
/// native reads: 0x411FA0
pub(super) fn p304(s: &mut dyn Ser) -> Result<()> {
    s.bounded(5)?;
    Ok(())
}

/// 305 TGC_UpdateMapStatusOrderRequest (handler 0x757f30, process 0x75da60)
/// 316 TGC_UpdateMapStatusOrderDecree (handler 0x7528f0, process 0x75da60)
/// native reads: 0x411FA0, 0x7528F0, 0x757F30, 0x765F70
pub(super) fn p305(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.bounded(4)?;
    s.uint(32)?;
    Ok(())
}

/// 306 TGC_PlayerJoinOrderRequest (handler 0x758130, process 0x75da80)
/// 317 TGC_PlayerJoinOrderDecree (handler 0x7510e0, process 0x75da80)
/// native reads: 0x410B50, 0x74CD30, 0x76A810, 0x77BBA0
pub(super) fn p306(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    s.text()?;
    s.text()?;
    s.uint(32)?;
    s.net_addr()?;
    s.uint(32)?;
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 308 TGC_PlayerLeaveOrderRequest (handler 0x758570, process 0x75daa0)
/// 309 TGC_PlayerLeaveOrderDecree (handler 0x753b50, process 0x75daa0)
/// native reads: 0x411FA0, 0x765F70
pub(super) fn p308(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.bounded(13)?;
    Ok(())
}

/// 324 KKC_UpdateKingdomNameOrderRequest (handler 0x775100, process 0x75e4c0)
/// 328 KKC_UpdateKingdomNameOrderDecree (handler 0x76e830, process 0x75e4c0)
/// native reads: 0x410B50, 0x76E7D0
pub(super) fn p324(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    s.text()?;
    s.uint(1)?;
    Ok(())
}

/// 325 KKC_MidgameCreateAIsAndSessionPlayersOrderRequest (handler 0x7753f0, process 0x75de50)
/// 329 KKC_MidgameCreateAIsAndSessionPlayersOrderDecree (handler 0x76e9b0, process 0x75de50)
/// native reads: 0x410B50, 0x76E9B0, 0x7753F0
pub(super) fn p325(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.uint(32)?;
    Ok(())
}

/// 332 SelectionSetOrderRequest (handler 0x7b4d00, process 0x5f4450)
/// 333 SelectionSetOrderDecree (handler 0x7af7d0, process 0x5f4450)
/// 336 MGPurchaseSelectionSetOrderRequest (handler 0x936240, process 0x950780)
/// 340 MGPurchaseSelectionSetOrderDecree (handler 0x9345a0, process 0x950780)
/// native reads: 0x76A530
pub(super) fn p332(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::B0)?;
    Ok(())
}

/// 335 KKC_TellMGSwapPlayerKingdomOrderRequest (handler 0x935fa0, process 0x950900)
/// 339 KKC_TellMGSwapPlayerKingdomOrderDecree (handler 0x934490, process 0x950900)
/// native reads: 0x76A600
pub(super) fn p335(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::A8)?;
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 342 TGC_SetKingdomColorOrderRequest (handler 0x9479f0, process 0x75e220)
/// 348 TGC_SetKingdomColorOrderDecree (handler 0x946a80, process 0x75e220)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p342(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa60))?;
    Ok(())
}

/// 343 TGC_SetKingdomMergeIntoIDSOrderRequest (handler 0x947c20, process 0x75e250)
/// 349 TGC_SetKingdomMergeIntoIDSOrderDecree (handler 0x946120, process 0x75e250)
/// native reads: 0x410B50
pub(super) fn p343(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    Ok(())
}

/// 344 TGC_AddKingdomStartPropertyOrderRequest (handler 0x947ea0, process 0x75e370)
/// 350 TGC_AddKingdomStartPropertyOrderDecree (handler 0x946b60, process 0x75e370)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p344(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa2c))?;
    Ok(())
}

/// 346 TGC_AddKingdomAdditionalActorOrderRequest (handler 0x948300, process 0x75e3d0)
/// 352 TGC_AddKingdomAdditionalActorOrderDecree (handler 0x946320, process 0x75e3d0)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p346(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb39f58))?;
    Ok(())
}

/// 347 TGC_SetKingdomBonusStockpileOrderRequest (handler 0x948540, process 0x75e400)
/// 353 TGC_SetKingdomBonusStockpileOrderDecree (handler 0x946c40, process 0x75e400)
/// native reads: 0x410B50, 0x411FA0, 0x946A00
pub(super) fn p347(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3a660))?;
    s.uint(32)?;
    Ok(())
}

/// 354 MGHelpPopupShownOrderRequest (handler 0x94a8a0, process 0x9504f0)
/// 355 MGHelpPopupShownOrderDecree (handler 0x94a380, process 0x9504f0)
/// native reads: 0x411FA0, 0x94A300
pub(super) fn p354(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    s.def(Registry(0xb4a074))?;
    Ok(())
}

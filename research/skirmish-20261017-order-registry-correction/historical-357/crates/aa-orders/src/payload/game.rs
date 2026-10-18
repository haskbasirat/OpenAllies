//! In-game and lobby-core (descriptors 0..=99) order payload schemas, drafted from the original-reader traces
//! (research/phase8-orders/gen_schemas.py) and reviewed against the native
//! decoders named in each doc comment. Wire order is the native read order.
#![allow(clippy::unnecessary_wraps)]
use super::{Registry, Ser, ThingCheck};
use crate::Result;

pub(super) fn schema(descriptor: usize) -> Option<super::Schema> {
    Some(match descriptor {
        0 => p0,
        1 => p1,
        2 => p2,
        3 => p3,
        4 => p4,
        5 => p0,
        6 => p1,
        7 => p2,
        8 => p3,
        9 => p4,
        10 => p10,
        11 => p10,
        12 => p12,
        13 => p12,
        14 => p0,
        15 => p2,
        16 => p12,
        17 => p12,
        18 => p0,
        19 => p2,
        20 => p2,
        21 => p2,
        24 => p0,
        25 => p25,
        26 => p0,
        27 => p0,
        28 => p25,
        29 => p0,
        30 => p0,
        31 => p25,
        32 => p0,
        33 => p0,
        34 => p25,
        35 => p0,
        36 => p36,
        37 => p36,
        38 => p2,
        39 => p39,
        40 => p40,
        41 => p0,
        42 => p2,
        43 => p39,
        44 => p40,
        45 => p0,
        46 => p46,
        47 => p47,
        48 => p48,
        49 => p49,
        50 => p0,
        52 => p2,
        53 => p0,
        54 => p25,
        55 => p0,
        56 => p0,
        57 => p40,
        58 => p58,
        59 => p59,
        60 => p58,
        61 => p0,
        62 => p0,
        63 => p0,
        65 => p46,
        66 => p47,
        67 => p48,
        68 => p49,
        69 => p0,
        71 => p2,
        72 => p0,
        73 => p25,
        74 => p0,
        75 => p0,
        76 => p40,
        77 => p58,
        78 => p59,
        79 => p58,
        80 => p0,
        81 => p0,
        82 => p0,
        88 => p40,
        89 => p40,
        90 => p25,
        92 => p25,
        96 => p0,
        97 => p0,
        98 => p0,
        99 => p99,
        _ => return None,
    })
}

/// 0 TGC_LaunchOrderRequest (handler 0x4155c0, process 0x75d8d0)
/// 5 TGC_LaunchOrderDecree (handler 0x410af0, process 0x75d8d0)
/// 14 SelectionClearOrderRequest (handler 0x4396a0, process 0x5f44f0)
/// 18 SelectionClearOrderDecree (handler 0x438890, process 0x5f44f0)
/// 24 KKC_CheatFogOrderRequest (handler 0x5fb910, process 0x5fa200)
/// 26 KKC_CheatWinOrderRequest (handler 0x5fbf10, process 0x5fa2c0)
/// 27 KKC_CheatLoseOrderRequest (handler 0x5fc1f0, process 0x5fa310)
/// 29 KKC_CheatNoWaitOrderRequest (handler 0x5fc7f0, process 0x5fa3b0)
/// 30 KKC_CheatFogOrderDecree (handler 0x5faab0, process 0x4b29b0)
/// 32 KKC_CheatWinOrderDecree (handler 0x5facd0, process 0x74b3b0)
/// 33 KKC_CheatLoseOrderDecree (handler 0x5fade0, process 0x74b3b0)
/// 35 KKC_CheatNoWaitOrderDecree (handler 0x5fb050, process 0x4b29b0)
/// 41 TGC_NotifyHostHasEndedTheGameOrderRequest (handler 0x609f00, process 0x75dbf0)
/// 45 TGC_NotifyHostHasEndedTheGameOrderDecree (handler 0x609050, process 0x75dbf0)
/// 50 KKC_GamePlayerObserverOrderRequest (handler 0x617610, process 0x61ee60)
/// 53 KKC_VoteToResignOrderRequest (handler 0x617e10, process 0x61f530)
/// 55 TGC_GameSaveOrderRequest (handler 0x6138c0, process 0x75d8f0)
/// 56 TGC_TogglePausedOrderRequest (handler 0x613aa0, process 0x75d930)
/// 61 TGC_SpeedUpOrderRequest (handler 0x614560, process 0x75dac0)
/// 62 TGC_SpeedDownOrderRequest (handler 0x614740, process 0x75dad0)
/// 63 TGC_NotifySelectedKeepPlayingOrderRequest (handler 0x614920, process 0x75dbd0)
/// 69 KKC_GamePlayerObserverOrderDecree (handler 0x60ee60, process 0x61ee60)
/// 72 KKC_VoteToResignOrderDecree (handler 0x60ef40, process 0x61f530)
/// 74 TGC_GameSaveOrderDecree (handler 0x60f0a0, process 0x75d8f0)
/// 75 TGC_TogglePausedOrderDecree (handler 0x60f100, process 0x75d930)
/// 80 TGC_SpeedUpOrderDecree (handler 0x60f1f0, process 0x75dac0)
/// 81 TGC_SpeedDownOrderDecree (handler 0x60f250, process 0x75dad0)
/// 82 TGC_NotifySelectedKeepPlayingOrderDecree (handler 0x60f2b0, process 0x75dbd0)
/// 96 TGC_StartLaunchCountdownOrderRequest (handler 0x6525b0, process 0x75db90)
/// 97 TGC_StartLaunchCountdownOrderDecree (handler 0x651b70, process 0x75db90)
/// 98 TGC_SetSessionEditorOrderRequest (handler 0x65fd00, process 0x75da00)
/// native reads: none (empty payload)
pub(super) fn p0(_s: &mut dyn Ser) -> Result<()> {
    Ok(())
}

/// 1 TGC_SetMapTypeOrderRequest (handler 0x413970, process 0x75da10)
/// 6 TGC_SetMapTypeOrderDecree (handler 0x412870, process 0x75da10)
/// native reads: 0x411FA0
pub(super) fn p1(s: &mut dyn Ser) -> Result<()> {
    s.bounded(6)?;
    Ok(())
}

/// 2 TGC_SetMessageOrderRequest (handler 0x413b90, process 0x75e980)
/// 7 TGC_SetMessageOrderDecree (handler 0x411db0, process 0x75e980)
/// 15 SelectionClearOrderRequest (handler 0x439920, process 0x5f4510)
/// 19 SelectionClearOrderDecree (handler 0x438c90, process 0x5f4510)
/// 20 KKC_HeardScriptCommandOrderRequest (handler 0x5c8360, process 0x61e420)
/// 21 KKC_HeardScriptCommandOrderDecree (handler 0x5c55d0, process 0x61e420)
/// 38 TGC_SetFilenameOrderRequest (handler 0x6098d0, process 0x75df90)
/// 42 TGC_SetFilenameOrderDecree (handler 0x6090b0, process 0x75df90)
/// 52 KKC_NotifyTriggerManagerOfVideoCompletionRequest (handler 0x617b40, process 0x61e1e0)
/// 71 KKC_NotifyTriggerManagerOfVideoCompletionDecree (handler 0x60f9a0, process 0x61e1e0)
/// native reads: 0x410B50
pub(super) fn p2(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    Ok(())
}

/// 3 TGC_SetMapTransferStatusOrderRequest (handler 0x413dc0, process 0x75dff0)
/// 8 TGC_SetMapTransferStatusOrderDecree (handler 0x411e90, process 0x75dff0)
/// native reads: 0x410B50, 0x411E90, 0x411FA0, 0x413DC0
pub(super) fn p3(s: &mut dyn Ser) -> Result<()> {
    s.bounded(4)?;
    s.text()?;
    s.uint(32)?;
    Ok(())
}

/// 4 TGC_SetNewStateOrderRequest (handler 0x413ff0, process 0x75d8a0)
/// 9 TGC_SetNewStateOrderDecree (handler 0x4128f0, process 0x75d8a0)
/// native reads: 0x410B50, 0x411FA0, 0x7847B0
pub(super) fn p4(s: &mut dyn Ser) -> Result<()> {
    s.bounded(6)?;
    s.bounded(16)?;
    s.uint(1)?;
    s.text()?;
    s.uint(32)?;
    s.uint(1)?;
    s.bounded(7)?;
    s.uint(1)?;
    s.uint(1)?;
    s.bounded(16)?;
    s.uint(32)?;
    s.uint(1)?;
    Ok(())
}

/// 10 TGC_SetPlayerNameOrderRequest (handler 0x41dd60, process 0x75e180)
/// 11 TGC_SetPlayerNameOrderDecree (handler 0x41d490, process 0x75e180)
/// native reads: 0x410B50, 0x765F70
pub(super) fn p10(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.text()?;
    Ok(())
}

/// 12 SelectionAddOrderRequest (handler 0x439160, process 0x5f4490)
/// 13 SelectionRemoveOrderRequest (handler 0x439400, process 0x5f44c0)
/// 16 SelectionAddOrderDecree (handler 0x438690, process 0x5f4490)
/// 17 SelectionRemoveOrderDecree (handler 0x438790, process 0x5f44c0)
/// native reads: 0x76A530
pub(super) fn p12(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::B0)?;
    Ok(())
}

/// 25 KKC_CheatMoneyOrderRequest (handler 0x5fbbf0, process 0x5fa240)
/// 28 KKC_CheatExperienceOrderRequest (handler 0x5fc4d0, process 0x5fa360)
/// 31 KKC_CheatMoneyOrderDecree (handler 0x5fabb0, process 0x5fa240)
/// 34 KKC_CheatExperienceOrderDecree (handler 0x5faef0, process 0x74b3b0)
/// 54 KKC_SetGameSpeedOrderRequest (handler 0x6136a0, process 0x75d810)
/// 73 KKC_SetGameSpeedOrderDecree (handler 0x60f020, process 0x75d810)
/// 90 TGC_SetMapSeedOrderRequest (handler 0x643560, process 0x75da40)
/// 92 TGC_SetMapSeedOrderDecree (handler 0x641c80, process 0x75da40)
/// native reads: 0x5FABB0, 0x5FAEF0, 0x5FBBF0, 0x5FC4D0, 0x60F020, 0x6136A0, 0x641C80, 0x643560
pub(super) fn p25(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    Ok(())
}

/// 36 TGC_VoteToKickPlayerOrderRequest (handler 0x5ffe50, process 0x75daf0)
/// 37 TGC_VoteToKickPlayerOrderDecree (handler 0x5fdd70, process 0x75daf0)
/// native reads: 0x5FDD70, 0x5FFE50, 0x765F70
pub(super) fn p36(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.uint(1)?;
    Ok(())
}

/// 39 TGC_SetDynamicCampaignOrderRequest (handler 0x609b00, process 0x75dfc0)
/// 43 TGC_SetDynamicCampaignOrderDecree (handler 0x609330, process 0x75dfc0)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p39(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3aa4c))?;
    Ok(())
}

/// 40 TGC_SetNewDynamicCampaignOrderRequest (handler 0x609d10, process 0x75da20)
/// 44 TGC_SetNewDynamicCampaignOrderDecree (handler 0x608fc0, process 0x75da20)
/// 57 TGC_SetPausedOrderRequest (handler 0x613c80, process 0x75d940)
/// 76 TGC_SetPausedOrderDecree (handler 0x60f160, process 0x75d940)
/// 88 KKC_SkipCutsceneOrderRequest (handler 0x6230d0, process 0x61e620)
/// 89 KKC_SkipCutsceneOrderDecree (handler 0x621a90, process 0x61e620)
/// native reads: 0x608FC0, 0x609D10, 0x60F160, 0x613C80, 0x621A90, 0x6230D0
pub(super) fn p40(s: &mut dyn Ser) -> Result<()> {
    s.uint(1)?;
    Ok(())
}

/// 46 KKC_TellActorAttackOrderRequest (handler 0x616ae0, process 0x61e390)
/// 65 KKC_TellActorAttackOrderDecree (handler 0x60ec50, process 0x61e390)
/// native reads: 0x76A5A0
pub(super) fn p46(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::B4)?;
    s.thing(ThingCheck::B4)?;
    Ok(())
}

/// 47 KKC_TellActorMoveOrderRequest (handler 0x616d80, process 0x61e830)
/// 66 KKC_TellActorMoveOrderDecree (handler 0x60f830, process 0x61e830)
/// native reads: 0x410B50, 0x60F760
pub(super) fn p47(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 48 KKC_TellActorAbilityOrderRequest (handler 0x617060, process 0x61e100)
/// 67 KKC_TellActorAbilityOrderDecree (handler 0x610ab0, process 0x61e100)
/// native reads: 0x410B50, 0x411FA0
pub(super) fn p48(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.def(Registry(0xb3ab10))?;
    s.text()?;
    Ok(())
}

/// 49 GamePlayerKingdomRequest (handler 0x617370, process 0x61ed70)
/// 68 GamePlayerKingdomDecree (handler 0x60ed60, process 0x61ed70)
/// native reads: 0x76A600
pub(super) fn p49(s: &mut dyn Ser) -> Result<()> {
    s.thing(ThingCheck::A8)?;
    Ok(())
}

/// 58 TGC_PublicChatOrderRequest (handler 0x613ea0, process 0x75e070)
/// 60 TGC_TeamChatOrderRequest (handler 0x614310, process 0x75e100)
/// 77 TGC_PublicChatOrderDecree (handler 0x60fb00, process 0x75e070)
/// 79 TGC_TeamChatOrderDecree (handler 0x60fd10, process 0x75e100)
/// native reads: 0x410B50, 0x60FB00, 0x60FD10, 0x613EA0, 0x614310
pub(super) fn p58(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    s.text()?;
    Ok(())
}

/// 59 TGC_PrivateChatOrderRequest (handler 0x6140f0, process 0x75e0b0)
/// 78 TGC_PrivateChatOrderDecree (handler 0x60fc20, process 0x75e0b0)
/// native reads: 0x410B50, 0x60F7D0, 0x765F70
pub(super) fn p59(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(32)?;
    s.text()?;
    s.player()?;
    Ok(())
}

/// 99 TGC_SetPlayerReadyStatusOrderRequest (handler 0x65fee0, process 0x75db40)
/// native reads: 0x65FEE0, 0x765F70
pub(super) fn p99(s: &mut dyn Ser) -> Result<()> {
    s.player()?;
    s.uint(32)?;
    Ok(())
}

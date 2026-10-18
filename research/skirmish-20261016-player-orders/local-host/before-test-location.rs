use super::*;
use crate::local_bootstrap::{Guid, InitialProperties, LocalBootstrap, Notifications, ResetOs};
struct Os;
impl ResetOs for Os {
    fn get_tick_count(&mut self) -> crate::Result<u32> {
        Ok(0x12345678)
    }
    fn co_create_guid(&mut self) -> crate::Result<Guid> {
        Ok(Guid {
            data1: 1,
            data2: 2,
            data3: 3,
            data4: [4; 8],
        })
    }
    fn sync_log_gate(&mut self) -> crate::Result<bool> {
        panic!("fresh reset synchronizer is disabled")
    }
}
struct Timer;
impl crate::local_host::MillisecondTimer for Timer {
    fn milliseconds(&mut self) -> std::result::Result<u32, String> {
        Ok(1000)
    }
}
#[derive(Default)]
struct RequiredEffects {
    diagnostics: Vec<Diagnostic>,
    gates: usize,
}
impl Effects for RequiredEffects {
    fn sync_log_gate(&mut self, _: Log) -> Result<bool> {
        self.gates += 1;
        Ok(true)
    } //explicit OS gate fixture
    fn diagnostic(&mut self, d: Diagnostic) -> Result<()> {
        self.diagnostics.push(d);
        Ok(())
    } //explicit warning sink fixture
    fn stop_film(&mut self, _: &mut PlayersBorrow<'_>) -> Result<()> {
        required("actual film-stop world effects")
    }
    fn set_admin_abort(&mut self) -> Result<()> {
        required("actual admin2E5 abort owner")
    }
    fn valid(
        &mut self,
        _: &mut PlayersBorrow<'_>,
        _: &mut u32,
        _: &CurrentClient,
        _: Validity,
        _: usize,
        _: &[Field],
    ) -> Result<bool> {
        required("actual native request validity")
    }
    fn process(
        &mut self,
        _: &mut PlayersBorrow<'_>,
        _: &mut LocalClientBorrow<'_>,
        _: &mut u32,
        _: &CurrentClient,
        _: usize,
        _: &mut Vec<Field>,
    ) -> Result<()> {
        required("actual gameplay decree")
    }
}
struct Ctx;
impl aa_orders::payload::SchemaContext for Ctx {
    fn registry_count(&self, _: aa_orders::payload::Registry) -> Result<usize> {
        required("not read by SetPlayer")
    }
    fn team_command_flags(&self, _: u32) -> Result<u32> {
        required("not read by SetPlayer")
    }
    fn definition_kind(&self, _: aa_orders::payload::Registry, _: u32) -> Result<u32> {
        required("not read by SetPlayer")
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
impl aa_orders::payload::Resolver for Ctx {
    fn thing(&self, _: aa_orders::payload::ThingCheck, _: u16) -> Option<u16> {
        panic!("not read")
    }
    fn player(&self, _: u32) -> bool {
        panic!("SetPlayer uint32 has no serialization resolver")
    }
    fn definition(&self, _: aa_orders::payload::Registry, _: u32) -> bool {
        panic!("not read")
    }
    fn named(&self, _: aa_orders::payload::NameKind, _: &[u16]) -> Option<Vec<u16>> {
        panic!("not read")
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_local_roster_client_admin_and_logs_execute_set_player_without_copies() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, localization::Localization,
        settings::Settings,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let localization = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    for precision in [
        aa_numeric::Precision::Single24,
        aa_numeric::Precision::Double53,
        aa_numeric::Precision::Extended64,
    ] {
        let mut owner = LocalBootstrap::from_native_constructors();
        owner
            .reset_node_fresh(
                &mut Os,
                &InitialProperties::from_native_constructor(),
                &Notifications::new(0),
            )
            .unwrap();
        let player = owner
            .create_first_local_player(
                &"Local Order Host".encode_utf16().collect::<Vec<_>>(),
                &[],
                &localization,
            )
            .unwrap();
        let mut timer = crate::local_host::NodeTime::from_native_constructor(Timer);
        timer.begin_update_prefix(precision).unwrap();
        owner
            .finish_fresh_self_host(&catalog, &mut timer, precision)
            .unwrap();
        let mut loan = owner.runtime_borrow().unwrap();
        let admin_address = loan.admin as *const _ as usize;
        let sync_address = loan.sync_a as *const _ as usize;
        let mut effects = RequiredEffects::default();
        let mut world = World {
            players: loan.players,
            local_client: loan.local_client,
            sync_a: loan.sync_a,
            sync_s: loan.sync_s,
            effects: &mut effects,
        };
        assert!(world.local_client.is_host_self());
        assert_eq!(world.local_client.player_2c(), Some(player.id()));
        // Actual ctor-null280 exercises native security rejection, retaining284.
        {
            let (slot, client) = loan.admin.ordering_context();
            assert_eq!(
                world.set_player(slot, client, player.id()).unwrap(),
                PlayerOutcome::SecurityRejected
            );
            assert_eq!(*slot, 0);
        }
        assert_eq!(world.effects.diagnostics, [Diagnostic::SecurityBreach]);
        // Real own request stream/history, real280=SelfNode established by752E60,
        // actual75D710 binds the same Node2C then284; per-peer suffix copies/clears.
        loan.admin
            .emit_request(
                true,
                320,
                &[Field::Uint {
                    width: 32,
                    value: player.id(),
                }],
                player.id(),
                &Ctx,
            )
            .unwrap();
        loan.admin.seal_requests().unwrap();
        let begin = loan.admin.requests.begin();
        let end = loan.admin.requests.end();
        assert!(begin < end);
        loan.admin
            .host_execute_owned_requests(
                world.local_client.player_2c(),
                &mut [],
                &mut world,
                &Ctx,
                &Ctx,
                &mut aa_orders::Gate::default(),
            )
            .unwrap();
        assert_eq!(loan.admin.requests.begin(), end);
        assert_eq!(loan.admin.previous_ordering_player, Some(player.id()));
        assert_eq!(loan.admin.ordering_player(), None);
        assert_eq!(loan.admin.current_client(), &CurrentClient::Null);
        assert_eq!(world.local_client.player_2c(), Some(player.id()));
        assert_eq!(world.sync_a as *const _ as usize, sync_address);
        assert_eq!(loan.admin as *const _ as usize, admin_address);
        assert!(world.effects.gates > 0);
        let handle = world.players.find_order_player(player.id(), false).unwrap();
        assert_eq!(
            source(world.players.player_link(&handle)).unwrap(),
            PeerLink::Node
        );
    }
}

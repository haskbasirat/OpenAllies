//! Current GWorld+28 aliases its actual MapResource owner during one concrete
//! managed load. Constructor4F7D2F/60 and caller5A2E15/5A3030 establish it.
//!
//! The root owns the stable Box and this capability together. It must retire
//! the capability before binding a child that constructs/replaces GWorld.
#[derive(Clone, Copy, Debug)]
pub struct SameWorldSettings {
    owner_address: usize,
}
impl SameWorldSettings {
    /// Only the concrete one-World factory may discharge the native pointer-
    /// flow proof. This is not a public raw-value or caller-boolean constructor.
    pub(crate) fn from_constructed_world(owner: &Option<u32>) -> Result<Self, String> {
        if owner.is_none() {
            return Err("constructed World requires its actual MapResource mode owner".into());
        }
        Ok(Self {
            owner_address: std::ptr::from_ref(owner).addr(),
        })
    }
    pub(crate) fn read(self, owner: &Option<u32>) -> Result<Option<u32>, String> {
        if std::ptr::from_ref(owner).addr() != self.owner_address {
            return Err(
                "global GWorld settings alias belongs to a different actual mode owner".into(),
            );
        }
        let mode =
            owner.ok_or("global GWorld's bound MapResource mode owner became unavailable")?;
        Ok(Some(mode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_v14_current_owner_mode_rereads() {
        let proof:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-world-settings-alias/native-world-settings-alias.json"
        )).unwrap();
        assert_eq!(proof["cases"].as_array().unwrap().len(), 12);
        for case in proof["cases"].as_array().unwrap() {
            let mut owner = Box::new(Some(case["input"]["initial_mode"].as_u64().unwrap() as u32));
            let alias = SameWorldSettings::from_constructed_world(owner.as_ref()).unwrap();
            assert_eq!(
                alias.read(owner.as_ref()).unwrap().unwrap(),
                case["output"]["mode_reads"][0].as_u64().unwrap() as u32
            );
            *owner = Some(case["input"]["later_mode"].as_u64().unwrap() as u32);
            assert_eq!(
                alias.read(owner.as_ref()).unwrap().unwrap(),
                case["output"]["mode_reads"][1].as_u64().unwrap() as u32
            );
            let foreign = Box::new(Some(
                case["output"]["foreign_global_reads_foreign_mode"]
                    .as_u64()
                    .unwrap() as u32,
            ));
            assert!(alias.read(foreign.as_ref()).is_err());
        }
    }

    #[test]
    fn stable_owner_moves_and_reads_each_current_word_without_accepting_a_foreign_owner() {
        let mut mode = Box::new(Some(1));
        let alias = SameWorldSettings::from_constructed_world(mode.as_ref()).unwrap();
        assert_eq!(alias.read(mode.as_ref()).unwrap(), Some(1));
        for value in [0, 2, 4, u32::MAX] {
            *mode = Some(value);
            assert_eq!(alias.read(mode.as_ref()).unwrap(), Some(value));
            assert_eq!(
                crate::live_placement::read_settings_mode(
                    true,
                    Some(99),
                    crate::live_placement::SettingsMode::SameWorld(alias),
                    Some(mode.as_ref())
                )
                .unwrap(),
                Some(value)
            );
            assert_eq!(
                crate::live_placement::read_settings_mode(
                    false,
                    Some(99),
                    crate::live_placement::SettingsMode::Unbound,
                    Some(mode.as_ref())
                )
                .unwrap(),
                Some(value)
            );
        }
        let moved = (mode, alias);
        assert_eq!(moved.1.read(moved.0.as_ref()).unwrap(), Some(u32::MAX));
        let foreign = Box::new(Some(u32::MAX));
        assert!(
            moved
                .1
                .read(foreign.as_ref())
                .unwrap_err()
                .contains("different")
        );
        assert!(
            crate::live_placement::read_settings_mode(
                true,
                Some(u32::MAX),
                crate::live_placement::SettingsMode::SameWorld(moved.1),
                Some(foreign.as_ref())
            )
            .is_err()
        );
        assert!(
            crate::live_placement::read_settings_mode(
                true,
                Some(1),
                crate::live_placement::SettingsMode::SameWorld(moved.1),
                None
            )
            .is_err()
        );
        assert!(
            crate::live_placement::read_settings_mode(
                true,
                Some(1),
                crate::live_placement::SettingsMode::Unbound,
                Some(moved.0.as_ref())
            )
            .is_err()
        );
        assert_eq!(
            crate::live_placement::read_settings_mode(
                true,
                Some(1),
                crate::live_placement::SettingsMode::Null,
                Some(moved.0.as_ref())
            )
            .unwrap(),
            None
        );
        assert!(SameWorldSettings::from_constructed_world(&None).is_err());
        let (mut mode, alias) = moved;
        *mode = None;
        assert!(
            alias
                .read(mode.as_ref())
                .unwrap_err()
                .contains("unavailable")
        );
    }
}

//!Session773CE0: same12C lock, display148, real timer/diagnostic, then14C.
use super::*;
use aa_numeric::Precision;
#[path = "local_session_status_core.rs"]
mod core;
fn error(e: impl ToString) -> Error {
    missing(e.to_string())
}
pub trait Services {
    ///Actual global B396E0 timer, distinct from Node+50 cached time.
    fn milliseconds_59aba0(&mut self) -> Result<u32>;
    fn progress_diagnostic(&mut self, seconds_bits: u32, text: &[u16]) -> Result<()>;
}
pub(super) struct State {
    lock: Mutex<()>,
    text_148: Vec<u16>,
    word_14c: Option<u32>,
}
impl State {
    pub(super) fn constructed() -> Self {
        Self {
            lock: Mutex::new(()),
            text_148: Vec::new(),
            word_14c: None,
        }
    }
}
struct StatusError(Error);
impl From<aa_numeric::Error> for StatusError {
    fn from(e: aa_numeric::Error) -> Self {
        Self(error(e))
    }
}
struct Actual<'a, S> {
    text: &'a mut Vec<u16>,
    word: &'a mut Option<u32>,
    input: &'a [u16],
    current: &'a aa_runtime::localization::Localization,
    translate: u8,
    services: &'a mut S,
}
impl<S: Services> core::Fields for Actual<'_, S> {
    type Error = StatusError;
    fn display_store_148(&mut self) -> std::result::Result<(), StatusError> {
        let text = if self.translate != 0 {
            self.current
                .display(self.input)
                .map_err(|e| StatusError(error(e)))?
        } else {
            self.input.iter().copied().take_while(|c| *c != 0).collect()
        };
        *self.text = text;
        Ok(())
    }
    fn milliseconds(&mut self) -> std::result::Result<u32, StatusError> {
        self.services.milliseconds_59aba0().map_err(StatusError)
    }
    fn diagnostic(&mut self, seconds: u32) -> std::result::Result<(), StatusError> {
        self.services
            .progress_diagnostic(seconds, self.text)
            .map_err(StatusError)
    }
    fn store_14c(&mut self, word: u32) -> std::result::Result<(), StatusError> {
        *self.word = Some(word);
        Ok(())
    }
}
impl LocalBootstrap {
    ///The real source text is borrowed only on the native translation branch.
    ///A late time/diagnostic refusal retains new148 and the previous14C.
    pub fn update_status_773ce0(
        &mut self,
        current_text: &aa_runtime::localization::Localization,
        input: &[u16],
        word: u32,
        translate: u8,
        services: &mut impl Services,
        pc: Precision,
    ) -> Result<()> {
        let state = &mut self.session.status;
        let _guard = state
            .lock
            .lock()
            .map_err(|_| missing("actual Session12C status mutex poisoned"))?;
        core::update(
            &mut Actual {
                text: &mut state.text_148,
                word: &mut state.word_14c,
                input,
                current: current_text,
                translate,
                services,
            },
            word,
            pc,
        )
        .map_err(|e| e.0)
    }
    pub fn status_text_148(&self) -> &[u16] {
        &self.session.status.text_148
    }
    pub fn status_word_14c(&self) -> Result<u32> {
        self.session
            .status
            .word_14c
            .ok_or_else(|| missing("Session14C has no reached773CE0 writer"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Sink {
        timer_fails: bool,
        diagnostic_fails: bool,
        seen: Vec<Vec<u16>>,
    }
    impl Services for Sink {
        fn milliseconds_59aba0(&mut self) -> Result<u32> {
            if self.timer_fails {
                Err(missing("required timer"))
            } else {
                Ok(0x80000001)
            }
        }
        fn progress_diagnostic(&mut self, _: u32, text: &[u16]) -> Result<()> {
            self.seen.push(text.to_vec());
            if self.diagnostic_fails {
                Err(missing("required diagnostic"))
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn actual_session_status_retains_new_text_before_late_failure() {
        let mut owner = LocalBootstrap::from_native_constructors();
        assert!(owner.status_text_148().is_empty());
        assert!(owner.status_word_14c().is_err());
        //False translation never reads this table, including its missingkeys.
        let table = aa_runtime::localization::Localization::default();
        let mut sink = Sink {
            timer_fails: true,
            diagnostic_fails: false,
            seen: Vec::new(),
        };
        let first = "first".encode_utf16().collect::<Vec<_>>();
        assert!(
            owner
                .update_status_773ce0(&table, &first, 7, 0, &mut sink, Precision::Double53)
                .is_err()
        );
        assert_eq!(owner.status_text_148(), first);
        assert!(owner.status_word_14c().is_err());
        assert!(sink.seen.is_empty());
        sink.timer_fails = false;
        owner
            .update_status_773ce0(&table, &first, 7, 0, &mut sink, Precision::Double53)
            .unwrap();
        sink.diagnostic_fails = true;
        let later = "later".encode_utf16().collect::<Vec<_>>();
        assert!(
            owner
                .update_status_773ce0(&table, &later, 9, 0, &mut sink, Precision::Double53)
                .is_err()
        );
        assert_eq!(owner.status_text_148(), later);
        assert_eq!(owner.status_word_14c().unwrap(), 7);
        assert_eq!(sink.seen.last().unwrap(), &later);
        //A translation refusal precedes148, timer and diagnostic.
        let before = sink.seen.len();
        assert!(
            owner
                .update_status_773ce0(&table, &first, 11, 128, &mut sink, Precision::Double53)
                .is_err()
        );
        assert_eq!(owner.status_text_148(), later);
        assert_eq!(owner.status_word_14c().unwrap(), 7);
        assert_eq!(sink.seen.len(), before);
    }
}

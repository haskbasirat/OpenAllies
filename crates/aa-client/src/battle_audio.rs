//! Native first-tick selection reaches the actual client audio device here.
//! Backend acknowledgement is distinct from verified audible playback.
use crate::audio::Audio;
use aa_runtime::live_first_tick::{AudioOutput,MusicRequest,Playback};

impl AudioOutput for Audio {
    fn play_747630(&mut self,request:&MusicRequest)->Result<Playback,String> {
        Ok(if self.play_music(request.audio.id()) {Playback::Accepted} else {Playback::Declined})
    }
}

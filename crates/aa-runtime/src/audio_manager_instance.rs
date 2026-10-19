//! TGC_Audio / TGC_MilesAudio behaviour: part selection (745DD0, 744EB0),
//! per-part Miles start (74AD10 -> 749650/7496E0/74A270), the per-frame
//! volume/fade update (748790), the 3-D position update (7485B0/745D60) and
//! the pause/resume/stop/fade-out virtuals (7484C0/748500/748540/748060).
use super::*;

impl<R: Clone + PartialEq + std::fmt::Debug, X> Manager<R, X> {
    /// 74AD10 (vt+8): select on the first call, then start the current part
    /// on its channel kind, or mark the audio released.
    pub(super) fn play_74ad10(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        if self.inst(id)?.state_34 == 0 {
            self.select_745dd0(env, id)?;
        }
        let inst = self.inst(id)?;
        if inst.state_34 == 4 || inst.current_3c.is_none() {
            self.inst_mut(id)?.released_39 = true;
            return Ok(());
        }
        match self.kind_745850(id)? {
            0 => self.start_2d_749650(env, platform, id),
            1 => self.start_3d_7496e0(env, platform, id),
            2 => self.start_stream_74a270(env, platform, id),
            _ => Ok(()),
        }
    }
    /// 745DD0: leadin/fallout/main picks with UI+D4 draws, resource requests,
    /// the first part (744EB0), then delay/pitch/volume randomization.
    fn select_745dd0(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<()> {
        let pc = self.pc;
        let data = Arc::clone(&self.inst(id)?.data);
        let mut draw = || env.ui_random_d4();
        let leadin = data.leadin_46b0d0(&mut draw)?;
        if let Some(r) = &leadin {
            env.resource_request(r, 1, 1)?;
        }
        let mut draw = || env.ui_random_d4();
        let fallout = data.fallout_46b190(&mut draw)?;
        if let Some(r) = &fallout {
            env.resource_request(r, 1, 1)?;
        }
        let looping = self.inst(id)?.loop_38;
        let main = if !looping && data.control & 4 == 0 {
            let mut draw = || env.ui_random_d4();
            data.main_46b110(&mut draw)?
        } else {
            None
        };
        if let Some(r) = &main {
            env.resource_request(r, 1, 1)?;
        } else {
            //46B640 copies the files list; [B282F0] = 1.
            for r in &data.files {
                env.resource_request(r, 1, 1)?;
            }
        }
        {
            let inst = self.inst_mut(id)?;
            inst.leadin_40 = leadin;
            inst.fallout_44 = fallout;
            inst.main_48 = main;
        }
        self.advance_744eb0(env, id)?;
        if self.inst(id)?.state_34 != 4 {
            let mut draw = || env.ui_random_d4();
            let delay = st(&random_range(data.delay_ms, &mut draw, pc)?)?;
            let pitch = random_range(data.pitch_adjust, &mut draw, pc)?;
            let pitch = st(&e(pitch.add(&f(ONE)?, pc))?)?;
            //46B3A0: low = (volume_adjust + 1.0) stored, equal to 1.0 returns it.
            let low = st(&e(f(data.volume_adjust)?.add(&f(ONE)?, pc))?)?;
            let factor = if same(low, ONE) {
                f(low)?
            } else {
                let r = unit(draw()?, pc)?;
                e(e(r.mul(&e(f(ONE)?.sub(&f(low)?, pc))?, pc))?.add(&f(low)?, pc))?
            };
            let volume = st(&e(factor.mul(&f(data.volume)?, pc))?)?;
            let inst = self.inst_mut(id)?;
            inst.delay_68 = delay;
            inst.pitch_6c = pitch;
            inst.volume_70 = volume;
        }
        Ok(())
    }
    /// 744EB0: advance Start -> Leadin -> Main (repeated while looping) ->
    /// Fallout -> Complete until a part exists; sync-load it (vt+18).
    pub(super) fn advance_744eb0(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<()> {
        loop {
            if self.disabled_189 {
                let inst = self.inst_mut(id)?;
                inst.state_34 = 4;
                inst.current_3c = None;
                return Ok(());
            }
            let inst = self.inst_mut(id)?;
            match inst.state_34 {
                0 => inst.state_34 = 1,
                1 => inst.state_34 = 2,
                2 if !inst.loop_38 => inst.state_34 = 3,
                2 => {}
                3 => {
                    inst.state_34 = 4;
                    inst.current_3c = None;
                    return Ok(());
                }
                4 => {
                    inst.current_3c = None;
                    return Ok(());
                }
                _ => {}
            }
            let state = self.inst(id)?.state_34;
            match state {
                1 => {
                    let inst = self.inst_mut(id)?;
                    inst.current_3c = inst.leadin_40.clone();
                }
                2 => {
                    let current = match self.inst(id)?.main_48.clone() {
                        Some(r) => Some(r),
                        None => {
                            let data = Arc::clone(&self.inst(id)?.data);
                            let mut draw = || env.ui_random_d4();
                            data.main_46b110(&mut draw)?
                        }
                    };
                    self.inst_mut(id)?.current_3c = current;
                }
                3 => {
                    let inst = self.inst_mut(id)?;
                    inst.current_3c = inst.fallout_44.clone();
                }
                _ => {}
            }
            self.events.push(Event::Part { id, state });
            if let Some(r) = self.inst(id)?.current_3c.clone() {
                return env.resource_sync_load(&r);
            }
            let inst = self.inst_mut(id)?;
            if inst.loop_38 && inst.state_34 == 2 {
                //vt+24 745840.
                inst.loop_38 = false;
            }
        }
    }
    /// 748070 (vt+28): the next part after an end of sample.
    pub(super) fn next_part_748070(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<bool> {
        self.advance_744eb0(env, id)?;
        if self.inst(id)?.state_34 != 4 {
            self.play_74ad10(env, platform, id)?;
            return Ok(true);
        }
        Ok(false)
    }
    fn channel_of(&self, id: u32) -> Result<Option<Channel>> {
        Ok(self.inst(id)?.channel_18.map(|c| self.miles.channels[c].clone()))
    }
    /// The pitch step of 749650/7496E0: when +6C is not the 1.0 bit pattern,
    /// rate = _ftol(fild(rate) * pitch).
    fn pitched_rate(&self, rate: i32, pitch: u32) -> Result<i32> {
        let value = e(Binary::from_i32(rate).mul(&f(pitch)?, self.pc))?;
        e(value.truncate_i32())
    }
    /// 749650: 2-D sample start.
    fn start_2d_749650(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        let h = channel.sample_0c;
        let part = self.inst(id)?.current_3c.clone().ok_or("749650 current part is null")?;
        let Some(raw) = env.raw_data_59b030(&part)? else {
            return self.eos_2d_749510(env, platform, h);
        };
        platform.init_sample(h)?;
        platform.set_sample_file(h, raw, 0)?;
        platform.register_eos_callback(h, true)?;
        self.update_volume_748790(env, platform, id)?;
        let pitch = self.inst(id)?.pitch_6c;
        if pitch != ONE {
            let rate = platform.sample_playback_rate(h)?;
            platform.set_sample_playback_rate(h, self.pitched_rate(rate, pitch)?)?;
        }
        platform.start_sample(h)
    }
    /// 7496E0: 3-D sample start.
    fn start_3d_7496e0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        let h = channel.sample3d_10;
        let part = self.inst(id)?.current_3c.clone().ok_or("7496E0 current part is null")?;
        let Some(raw) = env.raw_data_59b030(&part)? else {
            return self.eos_3d_749550(env, platform, h);
        };
        self.position_745920(env, platform, id, false)?;
        platform.set_3d_sample_file(h, raw)?;
        platform.register_3d_eos_callback(h, true)?;
        self.update_volume_748790(env, platform, id)?;
        let range = self.inst(id)?.data.range;
        platform.set_3d_sample_distances(h, range[1], range[0])?;
        let p = self.inst(id)?.position_28;
        platform.set_3d_position(h, p)?;
        let pitch = self.inst(id)?.pitch_6c;
        if pitch != ONE {
            let rate = platform.sample_3d_playback_rate(h)?;
            platform.set_3d_sample_playback_rate(h, self.pitched_rate(rate, pitch)?)?;
        }
        platform.start_3d_sample(h)
    }
    /// 74A270: open a new stream for the part (the previous handle is closed
    /// later through vt+28/748C50), then start it.
    fn start_stream_74a270(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let Some(index) = self.inst(id)?.channel_18 else { return Ok(()) };
        let part = self.inst(id)?.current_3c.clone().ok_or("74A270 current part is null")?;
        let old = self.miles.channels[index].stream_14;
        if old != 0 {
            self.defer_close_748c50(old);
        }
        let driver = self.miles.channels[index].driver_18;
        let stream = platform.open_stream(driver, &part)?;
        self.miles.channels[index].stream_14 = stream;
        self.update_volume_748790(env, platform, id)?;
        platform.register_stream_callback(stream, true)?;
        platform.start_stream(stream)?;
        let subtitle = self.inst(id)?.data.subtitle_7c;
        if greater(&f(subtitle)?, &f(ZERO)?) && env.subtitle_owner()? {
            return Err("74A270 6371D0 subtitle display (B3974C+188) is not ported".into());
        }
        Ok(())
    }

    /// 748790 (vt+1C): volume with fades and the FOGGED test, to Miles.
    pub(super) fn update_volume_748790(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let pc = self.pc;
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        let (data, volume_70, position) = {
            let inst = self.inst(id)?;
            (Arc::clone(&inst.data), inst.volume_70, inst.position_28)
        };
        let base = self.volumes.channel_745450(channel.kind, data.category, pc)?;
        let mut volume = st(&e(base.mul(&f(volume_70)?, pc))?)?;
        if !env.application_active()? {
            volume = ZERO;
        }
        if data.control & 0x10 != 0 && data.world & 2 != 0 && data.world & 1 == 0 {
            if env.vworld_ready_43aaf0()? {
                if !env.fog_audible([position[0], position[1]])? {
                    volume = ZERO;
                }
            } else {
                volume = ZERO;
            }
        }
        let fade = self.inst(id)?.fade_64;
        match fade {
            0 => {
                let time = data.fade_time_s[0];
                if !greater(&f(time)?, &f(ZERO)?) {
                    self.inst_mut(id)?.fade_64 = 1;
                } else {
                    let timer = self.fade_timer(env, id)?;
                    let elapsed = fild_unsigned(timer.elapsed_59aba0(env)?, pc)?;
                    let t = e(e(elapsed.div(&f(time)?, pc))?.mul(&f(MILLI)?, pc))?;
                    if t.compare(&f(ONE)?) == std::cmp::Ordering::Less {
                        volume = st(&e(t.mul(&f(volume)?, pc))?)?;
                    } else {
                        let inst = self.inst_mut(id)?;
                        inst.fade_64 = 1;
                        inst.fade_timer_60 = None;
                    }
                }
            }
            2 => {
                let time = data.fade_time_s[1];
                if !greater(&f(time)?, &f(ZERO)?) {
                    self.inst_mut(id)?.fade_64 = 3;
                } else {
                    let timer = self.fade_timer(env, id)?;
                    let elapsed = fild_unsigned(timer.elapsed_59aba0(env)?, pc)?;
                    let t = e(e(elapsed.div(&f(time)?, pc))?.mul(&f(MILLI)?, pc))?;
                    let t = e(f(ONE)?.sub(&t, pc))?;
                    if greater(&t, &f(ZERO)?) {
                        volume = st(&e(t.mul(&f(volume)?, pc))?)?;
                    } else {
                        let inst = self.inst_mut(id)?;
                        inst.fade_64 = 3;
                        volume = ZERO;
                        inst.fade_timer_60 = None;
                    }
                }
            }
            _ => {}
        }
        match channel.kind {
            0 => platform.set_sample_volume_pan(channel.sample_0c, volume, 0x3f00_0000),
            1 => platform.set_3d_sample_volume(channel.sample3d_10, volume),
            2 => platform.set_stream_volume_pan(channel.stream_14, volume, 0x3f00_0000),
            _ => Ok(()),
        }
    }
    /// +60 lazily created 40C940 timer.
    fn fade_timer(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<Timer> {
        if let Some(timer) = self.inst(id)?.fade_timer_60 {
            return Ok(timer);
        }
        let timer = Timer::start(env)?;
        self.inst_mut(id)?.fade_timer_60 = Some(timer);
        Ok(timer)
    }
    /// 745920: the position, refreshed through vt+20 when it is (0,0,0) or
    /// when forced.
    fn position_745920(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32, force: bool) -> Result<[u32; 3]> {
        let p = self.inst(id)?.position_28;
        if p.iter().all(|&c| same(c, ZERO)) || force {
            self.update_3d_7485b0(env, platform, id)?;
        }
        Ok(self.inst(id)?.position_28)
    }
    /// 745D60 (TGC_Audio vt+20 prefix): an object-attached audio follows its
    /// object; it stops when the view world is not ready or the object is gone.
    fn refresh_745d60(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let (mode, object) = {
            let inst = self.inst(id)?;
            (inst.mode_1c, inst.object_20)
        };
        if mode >= MODE_GLOBAL {
            return Ok(());
        }
        if !env.vworld_ready_43aaf0()? {
            return self.stop_748540(platform, id);
        }
        match env.object_position(object)? {
            Some(position) => {
                self.inst_mut(id)?.position_28 = position;
                Ok(())
            }
            None => self.stop_748540(platform, id),
        }
    }
    /// 7485B0 (vt+20): refresh, then the Miles 3-D position, pulled toward the
    /// microphone by [B40048]+208 * AudioConsiderCenteredRadius.
    pub(super) fn update_3d_7485b0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        self.refresh_745d60(env, platform, id)?;
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        let pc = self.pc;
        let radius = env.uvar_f32("AudioConsiderCenteredRadius")?;
        let camera = env.camera()?.ok_or("7485B0 reads [B40048]+208 with no current camera")?;
        let p = self.inst(id)?.position_28;
        let mic = self.mic_170;
        let mut d = [ZERO; 3];
        for i in 0..3 {
            d[i] = st(&e(f(p[i])?.sub(&f(mic[i])?, pc))?)?;
        }
        let r = st(&e(f(camera.zoom)?.mul(&f(radius)?, pc))?)?;
        let (dx, dy, dz) = (f(d[0])?, f(d[1])?, f(d[2])?);
        let sum = e(e(e(dz.mul(&dz, pc))?.add(&e(dy.mul(&dy, pc))?, pc))?.add(&e(dx.mul(&dx, pc))?, pc))?;
        let distance = e(sum.sqrt(pc))?;
        let target = if distance.compare(&f(r)?) == std::cmp::Ordering::Less {
            mic
        } else {
            let (n, _) = normalize_4453a0(d, pc)?;
            let r = f(r)?;
            let sx = e(f(n[0])?.mul(&r, pc))?;
            let sy = e(f(n[1])?.mul(&r, pc))?;
            let sz = st(&e(f(n[2])?.mul(&r, pc))?)?;
            [
                st(&e(f(p[0])?.sub(&sx, pc))?)?,
                st(&e(f(p[1])?.sub(&sy, pc))?)?,
                st(&e(f(p[2])?.sub(&f(sz)?, pc))?)?,
            ]
        };
        platform.set_3d_position(channel.sample3d_10, target)
    }
    /// 7484C0 (vt+C).
    pub(super) fn pause_7484c0(&mut self, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        match self.kind_745850(id)? {
            0 => platform.stop_sample(channel.sample_0c),
            1 => platform.stop_3d_sample(channel.sample3d_10),
            2 => platform.pause_stream(channel.stream_14, 1),
            _ => Ok(()),
        }
    }
    /// 748500 (vt+10).
    pub(super) fn resume_748500(&mut self, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        match self.kind_745850(id)? {
            0 => platform.resume_sample(channel.sample_0c),
            1 => platform.resume_3d_sample(channel.sample3d_10),
            2 => platform.pause_stream(channel.stream_14, 0),
            _ => Ok(()),
        }
    }
    /// 748540 (vt+14): released, no part, callbacks removed, Miles stop.
    pub(super) fn stop_748540(&mut self, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        {
            let inst = self.inst_mut(id)?;
            inst.released_39 = true;
            inst.current_3c = None;
        }
        let Some(channel) = self.channel_of(id)? else { return Ok(()) };
        match self.kind_745850(id)? {
            0 => {
                platform.register_eos_callback(channel.sample_0c, false)?;
                platform.stop_sample(channel.sample_0c)
            }
            1 => {
                platform.register_3d_eos_callback(channel.sample3d_10, false)?;
                platform.stop_3d_sample(channel.sample3d_10)
            }
            2 => {
                platform.register_stream_callback(channel.stream_14, false)?;
                platform.pause_stream(channel.stream_14, 1)
            }
            _ => Ok(()),
        }
    }
    /// 748060 (vt+18): start the fade-out when a channel exists.
    pub(super) fn begin_fade_out_748060(&mut self, id: u32) -> Result<()> {
        let inst = self.inst_mut(id)?;
        if inst.channel_18.is_some() {
            inst.fade_64 = 2;
        }
        Ok(())
    }
    /// 749510 / 749550 / 749590: Miles end-of-sample/stream callbacks.
    pub fn eos_2d_749510(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, handle: u32) -> Result<()> {
        let found = self.find_by_handle(0, handle);
        self.eos(env, platform, found)
    }
    pub fn eos_3d_749550(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, handle: u32) -> Result<()> {
        let found = self.find_by_handle(1, handle);
        self.eos(env, platform, found)
    }
    pub fn eos_stream_749590(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, handle: u32) -> Result<()> {
        let found = self.find_by_handle(2, handle);
        self.eos(env, platform, found)
    }
    /// 748A70/748B10/748BB0: the listed audio whose channel holds `handle`.
    fn find_by_handle(&self, list: usize, handle: u32) -> Option<u32> {
        if handle == 0 {
            return None;
        }
        self.lists[list].iter().copied().find(|id| {
            self.instances.get(id).and_then(|i| i.channel_18).is_some_and(|c| {
                let c = &self.miles.channels[c];
                [c.sample_0c, c.sample3d_10, c.stream_14][list] == handle
            })
        })
    }
    fn eos(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, found: Option<u32>) -> Result<()> {
        let Some(id) = found else { return Ok(()) };
        if env.application_kind()? == 3 || !self.next_part_748070(env, platform, id)? {
            self.inst_mut(id)?.released_39 = true;
        }
        Ok(())
    }
}

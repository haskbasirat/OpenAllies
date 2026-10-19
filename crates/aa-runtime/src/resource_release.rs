//!Resource release and unload over the SAME ResourceManager (child of
//!resource_depot; native proof in research/development10/resource-release).
//!
//!5AA550 Resource::Release: delta1 InterlockedDecrement(+4), any other delta a
//!subtraction under manager+20; only an exact zero continues to 5AEE70.
//!5AEE70 ReleaseResource: loaded (bit0) goes to 5AC3D0. Otherwise 5AED90
//!removes the queued task ("removed from queue": nothing else happens); a
//!popped task logs, and if still unloaded logs "hasn't run yet", runs 5B1970
//!and stops unless the load completed meanwhile ("couldn't abort ... in
//!time: unloading" then 5AC3D0).
//!5AC3D0 CacheOrUnloadResource (under +20): bit3 clear and UVar54
//!ResourceCaching true admit to the ResourceCache (resource_cache.rs);
//!otherwise "Unloading", +3C, clear bit0, vf40, vf54, "Unloaded".
//!5ABFF0 UnloadResource (5ACA50's eligible entry): +20, +3C; if bit0: clear,
//!vf40, vf54, "Unloaded"; if bit5 (reread): 59BFA0 cache removal.
//!Class vf40/vf54 (actual vtables A454A8/A45C98/A45290/A42C38):
//!  Texture 5AA700 / 5A7C80, Model 5A6560 / 5A7CF0, Sequence 5A6530 / 5A7CF0,
//!  Sound 5AA700 / 59AFE0.
//!B401AC (the deferred-release owner 70EF50 constructs at startup) is live in
//!the process, so vf54 hands +40 (Ni) or +3C (Texture source) to 59D430 /
//!5A75D0, which store the field NULL; the object is freed later on the
//!renderer side. Rust drops the field's ownership at the same point.
use super::{Backend, Resources};
use crate::resource_objects::{Handle, Kind};
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;

///One reached native diagnostic call, with its fixed native arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseLog {
    ///5AEE70 404F60 0x656 "aborting load task %s (%d): removed from queue".
    AbortRemovedFromQueue { count: u32 },
    ///0x661 "aborting load task %s (%d): already popped from queue".
    AbortAlreadyPopped { count: u32 },
    ///0x667 "aborting load task %s (%d): hasn't run yet".
    AbortNotRun { count: u32 },
    ///0x672 "couldn't abort load task %s (%d) in time: unloading".
    AbortTooLate { count: u32 },
    ///5AC3D0 404F90 0x69B "Unloading %s".
    Unloading,
    ///5AC3D0/5ABFF0/5ABFB0 404F90 0x523 "Unloaded %s".
    Unloaded,
    ///5A7CF0 404F90(B38B98) NiResource.cpp 0x32C "[%s] Unloading Ni resource
    ///'%s' (ref %d)"; the first %s is the current thread's name (+50).
    NiUnloading { count: u32 },
    ///5A7C80 404F90(B38B98) 0x268 "Unloading texture resource '%s'".
    TextureUnloading,
}
///The constant native call arguments of a [`ReleaseLog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeLog {
    pub channel: u32,
    ///404F60 forwards at channel level >= 10, 404F90 at >= 20.
    pub level: u32,
    pub file: &'static str,
    pub line: u32,
    pub function: &'static str,
    pub format: &'static str,
}
const MANAGER: &str = ".\\Source\\resource\\ResourceManager.cpp";
const NI: &str = ".\\Source\\resource\\NiResource.cpp";
impl ReleaseLog {
    pub fn native(&self) -> NativeLog {
        let (channel, level, file, line, function, format) = match self {
            Self::AbortRemovedFromQueue { .. } => (0xb38b70, 10, MANAGER, 0x656, "KKC_ResourceManager::ReleaseResource", "aborting load task %s (%d): removed from queue"),
            Self::AbortAlreadyPopped { .. } => (0xb38b70, 10, MANAGER, 0x661, "KKC_ResourceManager::ReleaseResource", "aborting load task %s (%d): already popped from queue"),
            Self::AbortNotRun { .. } => (0xb38b70, 10, MANAGER, 0x667, "KKC_ResourceManager::ReleaseResource", "aborting load task %s (%d): hasn't run yet"),
            Self::AbortTooLate { .. } => (0xb38b70, 10, MANAGER, 0x672, "KKC_ResourceManager::ReleaseResource", "couldn't abort load task %s (%d) in time: unloading"),
            Self::Unloading => (0xb38b70, 20, MANAGER, 0x69b, "KKC_ResourceManager::CacheOrUnloadResource", "Unloading %s"),
            Self::Unloaded => (0xb38b70, 20, MANAGER, 0x523, "KKC_ResourceManager::UnloadResource", "Unloaded %s"),
            Self::NiUnloading { .. } => (0xb38b98, 20, NI, 0x32c, "KKC_NiResource::Unload", "[%s] Unloading Ni resource '%s' (ref %d)"),
            Self::TextureUnloading => (0xb38b98, 20, NI, 0x268, "KKC_TextureResource::Unload", "Unloading texture resource '%s'"),
        };
        NativeLog { channel, level, file, line, function, format }
    }
}

///[`ReleaseServices`](crate::data_pre_request::request::release::ReleaseServices)
///over the SAME manager with the caller's actual resource backend.
pub struct ManagerRelease<'b, B>(pub &'b mut B);
impl<B: Backend> crate::data_pre_request::request::release::ReleaseServices for ManagerRelease<'_, B> {
    fn release_14(&mut self, resources: &mut Resources<'_>, handle: &Handle, delta: u32) -> Result<()> {
        resources.release_14(handle, delta, self.0)
    }
}

///Run `body` between a guard's enter and leave; the leave still runs (and an
///earlier error wins) when the body fails, as the native SEH unwinds do.
fn guarded<B: Backend, R>(
    backend: &mut B,
    enter: fn(&mut B, &Handle) -> Result<()>,
    leave: fn(&mut B, &Handle) -> Result<()>,
    handle: &Handle,
    body: impl FnOnce(&mut B) -> Result<R>,
) -> Result<R> {
    enter(backend, handle)?;
    let result = body(backend);
    let cleanup = leave(backend, handle);
    let value = result?;
    cleanup?;
    Ok(value)
}
fn enter20<B: Backend>(b: &mut B, h: &Handle) -> Result<()> {
    b.enter_request(h)
}
fn leave20<B: Backend>(b: &mut B, h: &Handle) -> Result<()> {
    b.leave_request(h)
}
fn enter3c<B: Backend>(b: &mut B, h: &Handle) -> Result<()> {
    b.enter_unload_3c(h)
}
fn leave3c<B: Backend>(b: &mut B, h: &Handle) -> Result<()> {
    b.leave_unload_3c(h)
}

impl Resources<'_> {
    ///Resource vf14: Texture 5A6300 (writable-child delegation, else 5AA550);
    ///Model/Sequence/Sound 5AA550.
    pub fn release_14(&mut self, handle: &Handle, delta: u32, backend: &mut impl Backend) -> Result<()> {
        if handle.kind() == Kind::Texture
            && self.texture_last_location_writable(handle)? == Some(true)
            && self.with_entry(handle, |e| Ok(e.texture.as_ref().is_some_and(|t| t.writable_child40.is_some())))?
        {
            return Err("5A6300 Texture vf14 writable child delegation remains required".into());
        }
        let zero = if delta == 1 {
            self.with_entry(handle, |e| {
                e.requests4 = e.requests4.wrapping_sub(1);
                Ok(e.requests4 == 0)
            })?
        } else {
            guarded(backend, enter20, leave20, handle, |_| {
                self.with_entry(handle, |e| {
                    e.requests4 = e.requests4.wrapping_sub(delta);
                    Ok(e.requests4 == 0)
                })
            })?
        };
        if !zero {
            return Ok(());
        }
        self.release_resource_5aee70(handle, backend)
    }
    fn release_resource_5aee70(&mut self, handle: &Handle, backend: &mut impl Backend) -> Result<()> {
        if self.flags_08(handle)? & 1 == 0 {
            let task = self.with_entry(handle, |e| Ok(Arc::clone(&e.task)))?;
            //5AED90 under manager+20: 0x636 diagnostic, then the queue removal.
            let tasks = self.tasks;
            let removed = guarded(backend, enter20, leave20, handle, |b| {
                b.removing_from_queue_diagnostic(handle)?;
                tasks.remove_5aed90(&task)
            })?;
            let count = self.requests_04(handle)?;
            if removed {
                return backend.release_log(handle, &ReleaseLog::AbortRemovedFromQueue { count });
            }
            backend.release_log(handle, &ReleaseLog::AbortAlreadyPopped { count })?;
            if self.flags_08(handle)? & 1 == 0 {
                let count = self.requests_04(handle)?;
                backend.release_log(handle, &ReleaseLog::AbortNotRun { count })?;
                task.abort_5b1970(backend)?;
                if self.flags_08(handle)? & 1 == 0 {
                    return Ok(());
                }
            }
            let count = self.requests_04(handle)?;
            backend.release_log(handle, &ReleaseLog::AbortTooLate { count })?;
        }
        self.cache_or_unload_5ac3d0(handle, backend)
    }
    ///5AC3D0 KKC_ResourceManager::CacheOrUnloadResource.
    fn cache_or_unload_5ac3d0(&mut self, handle: &Handle, backend: &mut impl Backend) -> Result<()> {
        guarded(backend, enter20, leave20, handle, |b| {
            if self.flags_08(handle)? & 8 == 0 && b.resource_caching_54(handle)? != 0 && self.admit_59bf30(handle)? {
                return Ok(());
            }
            b.release_log(handle, &ReleaseLog::Unloading)?;
            guarded(b, enter3c, leave3c, handle, |b| {
                self.with_entry(handle, |e| {
                    e.flags8 &= !1;
                    Ok(())
                })?;
                self.unload_40(handle, b)?;
                self.unload_54(handle, b)?;
                b.release_log(handle, &ReleaseLog::Unloaded)
            })
        })
    }
    ///5ABFF0 KKC_ResourceManager::UnloadResource.
    pub(crate) fn unload_5abff0(&mut self, handle: &Handle, backend: &mut impl Backend) -> Result<()> {
        guarded(backend, enter20, leave20, handle, |b| {
            guarded(b, enter3c, leave3c, handle, |b| {
                if self.flags_08(handle)? & 1 != 0 {
                    self.with_entry(handle, |e| {
                        e.flags8 &= !1;
                        Ok(())
                    })?;
                    self.unload_40(handle, b)?;
                    self.unload_54(handle, b)?;
                    b.release_log(handle, &ReleaseLog::Unloaded)?;
                }
                if self.flags_08(handle)? & 0x20 != 0 {
                    self.cache_remove_59bfa0(handle)?;
                }
                Ok(())
            })
        })
    }
    fn admit_59bf30(&mut self, handle: &Handle) -> Result<bool> {
        handle.require_application(self.origin)?;
        let key = crate::resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager = self.manager.lock().map_err(|_| "Resource manager lock poisoned")?;
        let super::Manager { resources, cache, .. } = &mut *manager;
        let entry = resources.get_mut(&key).filter(|e| e.handle.same_identity(handle)).ok_or("59BF30 actual Resource allocation differs")?;
        cache.admit_59bf30(entry)
    }
    ///59BFA0 for the SAME manager's cached Resource.
    pub(crate) fn cache_remove_59bfa0(&mut self, handle: &Handle) -> Result<()> {
        handle.require_application(self.origin)?;
        let key = crate::resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager = self.manager.lock().map_err(|_| "Resource manager lock poisoned")?;
        let super::Manager { resources, cache, .. } = &mut *manager;
        let entry = resources.get_mut(&key).filter(|e| e.handle.same_identity(handle)).ok_or("59BFA0 actual Resource allocation differs")?;
        cache.remove_59bfa0(entry)
    }
    ///Class vf40.
    fn unload_40(&mut self, handle: &Handle, backend: &mut impl Backend) -> Result<()> {
        match handle.kind() {
            //5AA700: two stores, bit0 then bit1 cleared.
            Kind::Texture | Kind::Sound | Kind::TerrainBlock => self.with_entry(handle, |e| {
                e.flags8 &= !1;
                e.flags8 &= !2;
                Ok(())
            }),
            Kind::Model | Kind::Sequence => {
                //5A6560 only: bit17 detaches +40 first (73D4B0).
                if handle.kind() == Kind::Model && (self.flags_08(handle)? >> 17) & 1 != 0 {
                    backend.detach_model_root_73d4b0(handle)?;
                }
                self.with_entry(handle, |e| {
                    e.flags8 &= !1;
                    e.flags8 &= !2;
                    Ok(())
                })?;
                handle.require_application(self.origin)?;
                let key = crate::resource_map_cache::normalized_key(handle.logical_ids())?;
                let mut manager = self.manager.lock().map_err(|_| "Resource manager lock poisoned")?;
                let super::Manager { resources, ni_cache, .. } = &mut *manager;
                let entry = resources.get_mut(&key).filter(|e| e.handle.same_identity(handle)).ok_or("Ni vf40 actual Resource allocation differs")?;
                let slot = match (&mut entry.model, &mut entry.sequence) {
                    (Some(m), _) => &mut m.source3c,
                    (_, Some(s)) => &mut s.source3c,
                    _ => return Err("Ni vf40 requires an actual Model/Sequence allocation".into()),
                };
                if let Some(cached) = slot.take() {
                    if let Err(error) = ni_cache.remove_59dab0(&cached, handle) {
                        *slot = Some(cached);
                        return Err(error);
                    }
                }
                Ok(())
            }
        }
    }
    ///Class vf54.
    fn unload_54(&mut self, handle: &Handle, backend: &mut impl Backend) -> Result<()> {
        match handle.kind() {
            Kind::Model | Kind::Sequence => {
                let count = self.requests_04(handle)?;
                backend.release_log(handle, &ReleaseLog::NiUnloading { count })?;
                //59D430(B401AC,&+40): field NULL, object to the deferred list.
                self.with_entry(handle, |e| {
                    if let Some(m) = &mut e.model {
                        m.source40 = None;
                    } else if let Some(s) = &mut e.sequence {
                        s.source40 = None;
                    } else {
                        return Err("5A7CF0 requires an actual Ni allocation".into());
                    }
                    Ok(())
                })
            }
            Kind::Texture => {
                backend.release_log(handle, &ReleaseLog::TextureUnloading)?;
                if self.texture_last_location_writable(handle)? == Some(true)
                    && self.with_entry(handle, |e| Ok(e.texture.as_ref().is_some_and(|t| t.writable_child40.is_some())))?
                {
                    return Ok(());
                }
                //5A75D0(B401AC,&+3C): our reference moves to the deferred list.
                let source = self.with_entry(handle, |e| Ok(e.texture.as_mut().ok_or("5A7C80 requires an actual Texture allocation")?.source3c.take()))?;
                if let Some(source) = source {
                    if let Err(error) = self.release_source_texture(&source) {
                        self.with_entry(handle, |e| {
                            if let Some(t) = &mut e.texture {
                                t.source3c = Some(source);
                            }
                            Ok(())
                        })?;
                        return Err(error);
                    }
                }
                Ok(())
            }
            Kind::TerrainBlock => backend.terrain_unload_71b5f0(handle),
            Kind::Sound => {
                if self.with_entry(handle, |e| Ok(e.sound.as_ref().ok_or("59AFE0 requires an actual Sound allocation")?.streaming41))? != 0 {
                    return Ok(());
                }
                Err("59AFE0 non-streaming Sound unload requires the actual AudioManager vf20 release".into())
            }
        }
    }
}

#[cfg(test)]
#[path = "resource_release_tests.rs"]
mod tests;

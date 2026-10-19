//!Aspect Texture requests: 5A79F0 (direct, TextureSearchPath list, final
//!flags0 lookup) and the loaded-Model extra textures 46AE20 over the
//!Aspect+48 {data, +4C count, +50 capacity} vector (46ADD0/46A450).
//!Proof: research/mapload-20261018/aspect-extra-textures (actual 5A79F0,
//!46AE20 sequences and 46AF70, original and v14, all PC).
use super::{Input, Result};
use crate::catalog::Catalog;

///85A550 (directory) and 859B70 (file name) of one counted name: the last
///'/' or '\' (859940 over 00AB53E0) splits it; otherwise the last ':'
///(wcsrchr) keeps the colon in the directory; otherwise the directory is
///empty and the file is the whole name.
pub(super) fn split_85a550_859b70(name: &[u16]) -> Result<(Vec<u16>, Vec<u16>)> {
    if name.contains(&0) {
        return Err("85A550/859B70 embedded-NUL name is outside the admitted counted domain".into());
    }
    let separator = name.iter().rposition(|&c| c == 47 || c == 92);
    Ok(if let Some(i) = separator {
        (name[..i].to_vec(), name[i + 1..].to_vec())
    } else if let Some(i) = name.iter().rposition(|&c| c == 58) {
        (name[..=i].to_vec(), name[i + 1..].to_vec())
    } else {
        (vec![], name.to_vec())
    })
}

///5A79F0(name, owner). `search` is the owner's list (ObjectData+B4, source
///order: 425E80 appends) or None for a NULL owner. `lookup` is 5A77E0(name,
///flags). An empty path is skipped; a '.'-relative path needs 85A030
///normalization, which is not ported and stops explicitly.
pub(super) fn request_5a79f0<H>(
    name: &[u16],
    search: Option<&[String]>,
    mut lookup: impl FnMut(&[u16], u32) -> Result<Option<H>>,
) -> Result<Option<H>> {
    if let Some(found) = lookup(name, 1)? {
        return Ok(Some(found));
    }
    let mut file = None;
    for path in search.unwrap_or(&[]) {
        let path: Vec<u16> = path.encode_utf16().collect();
        if path.is_empty() {
            continue;
        }
        if path[0] == u16::from(b'.') {
            return Err(
                "5A79F0 '.'-relative TextureSearchPath requires actual85A030 normalization".into(),
            );
        }
        if file.is_none() {
            file = Some(split_85a550_859b70(name)?.1);
        }
        let candidate = crate::film_paths::join(&path, file.as_ref().unwrap());
        if let Some(found) = lookup(&candidate, 1)? {
            return Ok(Some(found));
        }
    }
    lookup(name, 0)
}

///GameData+4 when a Model is requested in play: 48D660 stores 85A550 of the
///file it processes at the start of EVERY 48D970 file and no other code
///writes it, so after 48DB60 it is the directory of the LAST native-load
///file (the index chain ends with Data-root translate.tgi: empty).
pub(super) fn game_data_folder_4(catalog: &Catalog<'_>) -> Result<String> {
    let last = catalog
        .actors
        .sources
        .last()
        .ok_or("GameData+4 requires the actual 48D970 native load order")?;
    let (directory, _) = split_85a550_859b70(&last.path.encode_utf16().collect::<Vec<_>>())?;
    String::from_utf16(&directory).map_err(|_| "GameData+4 directory is not portable UTF-16".into())
}

///One +48 element (469CF0 constructs only +4). +0 is the 46A180 Texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Slot<T> {
    ///Operator-new storage no store wrote (46A450 copies it verbatim).
    Unwritten,
    ///46A180 returned NULL (empty source name).
    Null,
    Texture(T),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Extra<T> {
    pub(super) texture: Slot<T>,
    ///+4: 5A55A0 raw Model texture name.
    pub(super) name: Vec<u16>,
}
///Aspect+48 data and +50 capacity (the +4C count is the Row's word).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Extras<T> {
    pub(super) data: Option<Vec<Extra<T>>>,
    pub(super) capacity: i32,
}
impl<T: Clone> Extras<T> {
    ///46A510/46A650: +48 null, +50 one.
    pub(super) fn constructed() -> Self {
        Self {
            data: None,
            capacity: 1,
        }
    }
    ///46A450: an existing vector never shrinks below four; old elements are
    ///copied (+0 verbatim, +4 assigned) and the new tail is constructed.
    fn reallocate_46a450(&mut self, requested: i32) -> Result<()> {
        let capacity = if self.data.is_some() && requested < 4 {
            4
        } else {
            requested
        };
        let bytes = i64::from(capacity) * 8 + 4;
        if capacity < 0 || bytes > i64::from(u32::MAX) {
            return Err("46A450 +48 allocation size outside the native 32-bit domain".into());
        }
        let mut data = Vec::new();
        data.try_reserve_exact(capacity as usize)
            .map_err(|e| e.to_string())?;
        data.resize(
            capacity as usize,
            Extra {
                texture: Slot::Unwritten,
                name: vec![],
            },
        );
        if let Some(old) = self.data.take() {
            let copied = usize::try_from(self.capacity).map_err(|_| "46A450 negative old capacity")?;
            if copied > old.len() || copied > data.len() {
                self.data = Some(old);
                return Err("46A450 copy exceeds +48 storage".into());
            }
            data[..copied].clone_from_slice(&old[..copied]);
        }
        self.data = Some(data);
        self.capacity = capacity;
        Ok(())
    }
    ///46ADD0(count): grow to count, double, or allocate the first array;
    ///+4C is always stored.
    pub(super) fn resize_46add0(&mut self, count_4c: &mut u32, count: i32) -> Result<()> {
        let capacity = self.capacity;
        if capacity.wrapping_mul(2) < count {
            self.reallocate_46a450(count)?;
        } else if capacity < count {
            self.reallocate_46a450(capacity.wrapping_mul(2))?;
        } else if self.data.is_none() {
            self.reallocate_46a450(capacity)?;
        }
        *count_4c = count as u32;
        Ok(())
    }
    pub(super) fn element(&self, index: usize) -> Result<&Extra<T>> {
        self.data
            .as_ref()
            .and_then(|d| d.get(index))
            .ok_or_else(|| "Aspect+48 element outside its native storage".into())
    }
    ///+48[index]+0 as a Resource that a vf call dereferences.
    pub(super) fn texture(&self, index: usize) -> Result<&T> {
        match &self.element(index)?.texture {
            Slot::Texture(t) => Ok(t),
            Slot::Null => Err("Aspect+48 extra Texture is NULL (native null dereference)".into()),
            Slot::Unwritten => Err("Aspect+48 extra Texture +0 was never stored".into()),
        }
    }
}

///What 46AE20 reaches outside the Aspect.
pub(super) trait ExtraServices {
    type Texture: Clone;
    ///5A4ED0: Model+7C.
    fn model_texture_count(&mut self) -> Result<i32>;
    ///5A5590 (+80 element +0 full name) and 5A55A0 (+4 raw name).
    fn model_texture(&mut self, index: usize) -> Result<(Vec<u16>, Vec<u16>)>;
    ///4763C0's GameData+4 at this call.
    fn folder(&mut self) -> Result<String>;
    ///5A79F0(name, Aspect+44).
    fn request_5a79f0(&mut self, name: &[u16]) -> Result<Option<Self::Texture>>;
    ///Texture vf10(delta, mode).
    fn request_10(&mut self, texture: &Self::Texture, delta: u32, mode: u32) -> Result<()>;
}

///46AE20(delta, mode). Returns the 5A4ED0 count it read. A differing +4C
///resizes the vector and resolves EVERY element (46A180 flag0, +0, then the
///raw +4 name) before its vf10; an equal count only repeats vf10. +54 is set
///last, also when the count is zero. An error leaves the reached prefix.
pub(super) fn extra_46ae20<S: ExtraServices>(
    extras: &mut Extras<S::Texture>,
    count_4c: &mut u32,
    requested_54: &mut bool,
    input: &Input,
    services: &mut S,
    delta: u32,
    mode: u32,
) -> Result<i32> {
    let count = services.model_texture_count()?;
    let same = *count_4c as i32 == count;
    if !same {
        extras.resize_46add0(count_4c, count)?;
    }
    let mut folder = None;
    for index in 0..count.max(0) as usize {
        if !same {
            let (full, raw) = services.model_texture(index)?;
            let source = String::from_utf16(&full)
                .map_err(|_| "46AE20 Model texture name is not portable UTF-16".to_owned())?;
            if folder.is_none() {
                folder = Some(services.folder()?);
            }
            let name = input.texture_name_46a180(&source, false, folder.as_ref().unwrap())?;
            let texture = match super::texture_request_name(&name)? {
                None => Slot::Null,
                Some(name) => match services.request_5a79f0(&name)? {
                    Some(t) => Slot::Texture(t),
                    None => Slot::Null,
                },
            };
            let data = extras
                .data
                .as_mut()
                .and_then(|d| d.get_mut(index))
                .ok_or("46AE20 element outside +48 storage")?;
            data.texture = texture;
            data.name = raw;
        }
        let texture = extras.texture(index)?.clone();
        services.request_10(&texture, delta, mode)?;
    }
    *requested_54 = true;
    Ok(count)
}

///What 46AF70 reaches.
pub(super) trait BlockServices {
    ///Model28 flags+8 bit0.
    fn model_loaded(&mut self) -> Result<bool>;
    ///Model vf18 (BlockTillLoaded).
    fn block_18(&mut self) -> Result<()>;
    fn requested_54(&mut self) -> Result<bool>;
    fn count_18(&mut self) -> Result<u32>;
    fn extra_46ae20(&mut self, delta: u32, mode: u32) -> Result<()>;
    ///B3A747 assertion latch.
    fn assertion_latched(&mut self) -> Result<bool>;
    ///855D30("m_extra_textures_requested", AspectData.cpp, 0x1C9).
    fn assert_extra_requested(&mut self) -> Result<()>;
}
///46AF70: block on an unloaded Model, request the extra textures once with
///(+18, 1), then assert +54 unless the latch is set.
pub(super) fn block_46af70(w: &mut impl BlockServices) -> Result<()> {
    if !w.model_loaded()? {
        w.block_18()?;
    }
    if !w.requested_54()? {
        let count = w.count_18()?;
        w.extra_46ae20(count, 1)?;
    }
    if !w.assertion_latched()? && !w.requested_54()? {
        w.assert_extra_requested()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "data_aspect_texture_tests.rs"]
mod tests;

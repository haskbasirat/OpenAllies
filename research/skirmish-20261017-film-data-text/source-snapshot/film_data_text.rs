//!59FDA0's856E90 memory TextStream, defaultC-locale856BF0 writer and877AC0
//!raw stream forwarding. The generic resource namespace/path authority belongs
//!to the same Application; positive DataFile lookups remain explicit there.
use crate::{
    film_resource::Resource,
    resource_depot::{Application, WorkPath},
};
use std::io::Write;
type Result<T> = std::result::Result<T, String>;

///859C60 removes from the LAST dot even when it is in a directory name;
///85AB30 then appends the actual A0CF30 extension literal, prefixed with a dot.
fn sidecar_name(name: &[u16]) -> Vec<u16> {
    let end = name
        .iter()
        .rposition(|&v| v == u16::from(b'.'))
        .unwrap_or(name.len());
    let mut result = name[..end].to_vec();
    result.extend(".TGI".encode_utf16());
    result
}

#[cfg(test)]
#[path = "film_data_text_tests.rs"]
mod tests;
#[cfg(windows)]
fn path_units(path: &std::path::Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().collect()
}
#[cfg(not(windows))]
fn path_units(path: &std::path::Path) -> Vec<u16> {
    path.to_string_lossy().encode_utf16().collect()
}

///Move-only856E90/8C62E0 owner. This is a TextStream with ONE memory child,
///not a fabricated DataFileResource tree, manager or duplicate film stream.
///The source application receipt cannot be supplied from an arbitrary path.
pub struct PreparedDataText {
    path: Option<WorkPath>,
    bytes: Option<Vec<u8>>,
}
impl PreparedDataText {
    pub fn prepare(app: &mut Application, film: &Resource) -> Result<Self> {
        let ids = film.metadata()?.ids().to_vec();
        let logical = sidecar_name(&ids);
        let path = app.prepare_data_text_path(&logical)?;
        if !film
            .depots()?
            .last()
            .is_some_and(|d| d.same(path.depot_id()))
        {
            return Err("59FC90 positive non-work last-depot filename owner required".into());
        }
        let physical = app.work_path(&ids)?;
        if sidecar_name(&path_units(&physical)) != path_units(path.path()) {
            return Err("59FC90 physical last-dot filename leaves bound workpath domain".into());
        }
        //856E90 default0x2010A|0x40000|0x20000 selects the real memory
        //child, no BOM.8C5FE0 opens a growable initial0x4000-byte allocation.
        Ok(Self {
            path: Some(path),
            bytes: Some(Vec::with_capacity(0x4000)),
        })
    }
    fn current(&self) -> Result<&Vec<u8>> {
        self.bytes
            .as_ref()
            .ok_or("DataText wrapper was released".into())
    }
    fn filename(&self) -> Result<&WorkPath> {
        self.path
            .as_ref()
            .ok_or("DataText filename was released".into())
    }
    ///4738C0's admitted logicalC-locale output.856BF0 writes a CR before
    ///EVERY LF, including one already preceded by CR. No blanket normalization.
    pub fn build_current(&mut self, logical: &[u8]) -> Result<()> {
        if logical.contains(&0) {
            return Err("DataText logical source contains an unbound embeddedNUL".into());
        }
        let bytes = self.bytes.as_mut().ok_or("DataText wrapper was released")?;
        for &byte in logical {
            if byte == b'\n' {
                bytes.push(b'\r');
            }
            bytes.push(byte);
        }
        Ok(())
    }
    pub fn ensure_existing_work(&mut self, app: &mut Application) -> Result<()> {
        self.current()?;
        self.filename()?.require_application(app)?;
        //The genuine absent result was obtained through the same namespace
        //and actual source/work filesystem. Native iVar5==NULL skips this block.
        Ok(())
    }
    ///877AC0->8C62C0 writes current length bytes to the actual open chunk.
    ///No count/filename prefix is added by this native child virtual.
    pub fn serialize_into(&mut self, resource: &mut Resource) -> Result<()> {
        resource.write(self.current()?)
    }
    ///859B30/DeleteFileW Boolean is deliberately ignored by native59FDA0.
    pub fn remove_separate_file(&mut self) -> Result<()> {
        self.current()?;
        let _ = self.filename()?.delete_existing()?;
        Ok(())
    }
    ///877A50's separate regular-file transport; caller reads current option
    ///only afterserialize/delete. No original profile path is accepted here.
    pub fn save_separate(&mut self) -> Result<()> {
        let bytes = self.current()?;
        let path = self.filename()?;
        path.validate_path()?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            //876C60's actual default0x2010A maps to no-share CREATE_ALWAYS.
            options.share_mode(0);
        }
        let mut file = options.open(path.path()).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        //876720->8766D0 invokes real FlushFileBuffers before CloseHandle.
        file.sync_all().map_err(|e| e.to_string())
    }
    ///Actual release retires the same sole child; subsequent use is refused.
    pub fn release(&mut self) -> Result<()> {
        self.current()?;
        drop(self.bytes.take());
        drop(self.path.take());
        Ok(())
    }
    pub fn bytes(&self) -> Result<&[u8]> {
        Ok(self.current()?.as_slice())
    }
}

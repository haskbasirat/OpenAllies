//!Borrowed selected Banner source/request kernel46D630/46D370/46CEE0.
//!Each pending allocation remains owned until its actual vector append.
pub(super) type Result<T> = std::result::Result<T, String>;
#[derive(Debug)]
pub(super) struct Core<R> {
    pub prefix: String,
    pub model: String,
    pub skin: Option<R>,
    pub planes: Vec<Plane<R>>,
    pub experience: Vec<usize>,
    pub pending: Option<Plane<R>>,
}
#[derive(Debug)]
pub(super) struct Plane<R> {
    pub function: i32,
    pub flashing: bool,
    pub name: String,
    pub decals: Vec<Decal<R>>,
    pub pending: Option<Decal<R>>,
}
#[derive(Debug)]
pub(super) struct Decal<R> {
    pub function: i32,
    pub flashing: bool,
    pub textures: Vec<R>,
}
pub(super) trait Source<R> {
    type Node: Clone;
    fn text(&mut self, node: &Self::Node, key: &str) -> Result<String>;
    fn flag(&mut self, node: &Self::Node, key: &str) -> Result<bool>;
    fn children(&mut self, node: &Self::Node, key: &str) -> Result<Vec<Self::Node>>;
    fn path(&mut self, format: &str, prefix: &str) -> Result<String>;
    fn texture(&mut self, path: &str) -> Result<Option<R>>;
}
///46D2C0 actual constructor registration;497220 is case-insensitive.
pub(super) fn function(name: &str) -> i32 {
    [
        "NONE",
        "EXPERIENCE",
        "ENGAGED",
        "ENTRENCHED",
        "HERO",
        "OUT_OF_SUPPLY",
        "ROUT",
        "UPKEEP_SHORTAGE",
        "BANNER_ICON",
        "BADGE",
    ]
    .iter()
    .position(|n| n.eq_ignore_ascii_case(name))
    .map_or(-1, |v| v as i32)
}
fn item<R, S: Source<R>>(s: &mut S, n: &S::Node, kind: &mut i32, flash: &mut bool) -> Result<()> {
    let name = s.text(n, "function")?;
    if name.is_empty() {
        return Err("46CDD0 missing Banner item function".into());
    }
    *kind = function(&name);
    if *kind < 0 {
        return Err("46CDD0 unknown Banner item function".into());
    }
    *flash = s.flag(n, "flashing")?;
    Ok(())
}
impl<R> Core<R> {
    pub fn constructed() -> Self {
        Self {
            prefix: String::new(),
            model: String::new(),
            skin: None,
            planes: vec![],
            experience: vec![],
            pending: None,
        }
    }
    pub fn load<S: Source<R>>(&mut self, root: &S::Node, s: &mut S) -> Result<()> {
        self.prefix = s.text(root, "resource_prefix")?;
        let model = s.text(root, "model")?;
        if model.is_empty() {
            return Err("46D695 missing Banner model".into());
        }
        self.model = s.path(&model, &self.prefix)?;
        if self.model.is_empty() {
            return Err("46D704 missing formatted Banner model".into());
        }
        let skin = s.text(root, "skin")?;
        if !skin.is_empty() {
            let path = s.path(&skin, &self.prefix)?;
            self.skin = s.texture(&path)?;
        }
        for node in s.children(root, "Plane")? {
            self.pending = Some(Plane {
                function: 10,
                flashing: false,
                name: String::new(),
                decals: vec![],
                pending: None,
            });
            let plane = self.pending.as_mut().unwrap();
            item(s, &node, &mut plane.function, &mut plane.flashing)?;
            if matches!(plane.function, 8 | 9) {
                return Err("46D370 Banner icon/badge cannot be represented by Plane".into());
            }
            plane.name = s.text(&node, "name")?;
            if plane.name.is_empty() {
                return Err("46D370 Banner Plane missing name".into());
            }
            for node in s.children(&node, "Decal")? {
                plane.pending = Some(Decal {
                    function: 10,
                    flashing: false,
                    textures: vec![],
                });
                let decal = plane.pending.as_mut().unwrap();
                item(s, &node, &mut decal.function, &mut decal.flashing)?;
                for node in s.children(&node, "Texture")? {
                    let name = s.text(&node, "texture")?;
                    if name.is_empty() {
                        return Err("46CEE0 Banner decal texture not specified".into());
                    }
                    let name = s.path(&name, &self.prefix)?;
                    let texture = s
                        .texture(&name)?
                        .ok_or("46CEE0 hard Texture factory returned null")?;
                    decal.textures.push(texture);
                }
                if !matches!(decal.function, 8 | 9) && decal.textures.is_empty() {
                    return Err("46CEE0 Banner decal requires at least one Texture".into());
                }
                if decal.function != 1 && decal.textures.len() > 1 {
                    return Err("46CEE0 non-experience Banner decal has multiple Textures".into());
                }
                plane.decals.push(plane.pending.take().unwrap());
            }
            let index = self.planes.len();
            self.planes.push(self.pending.take().unwrap());
            if self.planes[index].function == 1 {
                self.experience.push(index);
            }
        }
        Ok(())
    }
    ///All three native loops capture their own count. The canonical source
    ///vectors stay immutable throughout these Resource-only callbacks.
    pub fn request(
        &self,
        delta: u32,
        mode: u32,
        mut call: impl FnMut(&R, u32, u32) -> Result<()>,
    ) -> Result<()> {
        let count = self.planes.len();
        for i in 0..count {
            let plane = &self.planes[i];
            let count = plane.decals.len();
            for j in 0..count {
                let decal = &plane.decals[j];
                let count = decal.textures.len();
                for k in 0..count {
                    call(&decal.textures[k], delta, mode)?;
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "data_banner_fields_tests.rs"]
mod tests;

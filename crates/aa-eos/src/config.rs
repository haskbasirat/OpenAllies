//! The product's IDs and the client secret, found at run time.
//!
//! None of these is ever in the repository. The IDs are in `openallies-ids.txt`
//! (`KEY=value` lines, `#` comments); the secret in `client-secret.txt` beside it
//! or the variable `AA_EOS_CLIENT_SECRET`. The folder is `AA_EOS_CONFIG_DIR`, else
//! `eos/` beside the program, else `target/eos/` of a folder above the program or
//! the current directory (a developer's tree).
use std::fmt;
use std::path::{Path, PathBuf};

pub const IDS_FILE: &str = "openallies-ids.txt";
pub const SECRET_FILE: &str = "client-secret.txt";

/// The client secret: shown by nothing, wiped when dropped.
pub struct Secret(String);

impl Secret {
    pub(crate) fn new(s: String) -> Self {
        Self(s)
    }
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<secret>")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(std::mem::take(&mut self.0).into_bytes());
    }
}

/// Overwrites a copy of the secret before it is freed.
pub(crate) fn wipe(mut bytes: Vec<u8>) {
    for b in &mut bytes {
        // SAFETY: a valid, exclusive reference; volatile so it is not removed as
        // a write to memory about to be freed.
        unsafe { std::ptr::write_volatile(b, 0) };
    }
}

/// What a platform needs to start.
#[derive(Debug)]
pub struct Config {
    pub product_id: String,
    pub sandbox_id: String,
    pub deployment_id: String,
    pub client_id: String,
    pub client_secret: Secret,
    /// Shown to Epic as the product's name and version.
    pub product_name: String,
    pub product_version: String,
    /// Where the SDK keeps its cache: an existing, writable, absolute folder.
    pub cache_dir: Option<PathBuf>,
}

/// Why online play has no settings.
#[derive(Debug)]
pub enum ConfigError {
    NoIds { searched: Vec<PathBuf> },
    MissingId { file: PathBuf, key: &'static str },
    NoSecret { dir: PathBuf },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // the first places are the ones a person would use; a message box
            // has no room for every folder above the program
            ConfigError::NoIds { searched } => {
                write!(f, "The online settings file {IDS_FILE} was not found. Looked in:")?;
                for p in searched.iter().take(2) {
                    write!(f, " {}", p.display())?;
                }
                if searched.len() > 2 {
                    write!(f, " and {} folders above them", searched.len() - 2)?;
                }
                Ok(())
            }
            ConfigError::MissingId { file, key } => write!(f, "{} has no {key}", file.display()),
            ConfigError::NoSecret { dir } => write!(f, "The client secret was not found: no {SECRET_FILE} in {} and no AA_EOS_CLIENT_SECRET", dir.display()),
        }
    }
}
impl std::error::Error for ConfigError {}

fn places(exe_dir: Option<&Path>, cwd: Option<&Path>, override_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(d) = override_dir {
        out.push(d);
    }
    if let Some(e) = exe_dir {
        out.push(e.join("eos"));
    }
    for start in [exe_dir, cwd].into_iter().flatten() {
        for up in start.ancestors() {
            let p = up.join("target").join("eos");
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// One `KEY=value` of the IDs file.
fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().map(str::trim).filter(|l| !l.starts_with('#')).find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim()).filter(|v| !v.is_empty())
    })
}

impl Config {
    /// The settings of this machine. `name` and `version` are shown to Epic.
    pub fn load(name: &str, version: &str, cache_dir: Option<PathBuf>) -> Result<Self, ConfigError> {
        let exe = std::env::current_exe().ok();
        let exe_dir = exe.as_deref().and_then(Path::parent);
        let cwd = std::env::current_dir().ok();
        let searched = places(exe_dir, cwd.as_deref(), std::env::var_os("AA_EOS_CONFIG_DIR").map(PathBuf::from));
        let Some(dir) = searched.iter().find(|d| d.join(IDS_FILE).is_file()).cloned() else {
            return Err(ConfigError::NoIds { searched });
        };
        let file = dir.join(IDS_FILE);
        let text = std::fs::read_to_string(&file).map_err(|_| ConfigError::NoIds { searched: vec![dir.clone()] })?;
        let id = |key: &'static str| value(&text, key).map(str::to_string).ok_or(ConfigError::MissingId { file: file.clone(), key });
        let (product_id, sandbox_id, deployment_id, client_id) = (id("EOS_PRODUCT_ID")?, id("EOS_SANDBOX_ID")?, id("EOS_DEPLOYMENT_ID")?, id("EOS_CLIENT_ID")?);
        let secret = std::env::var("AA_EOS_CLIENT_SECRET").ok().or_else(|| std::fs::read_to_string(dir.join(SECRET_FILE)).ok());
        let secret = secret.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).ok_or(ConfigError::NoSecret { dir })?;
        Ok(Self {
            product_id,
            sandbox_id,
            deployment_id,
            client_id,
            client_secret: Secret::new(secret),
            product_name: name.to_string(),
            product_version: version.to_string(),
            cache_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_shows_in_nothing() {
        let c = Config {
            product_id: "p".into(),
            sandbox_id: "s".into(),
            deployment_id: "d".into(),
            client_id: "c".into(),
            client_secret: Secret::new("hunter2".into()),
            product_name: "n".into(),
            product_version: "v".into(),
            cache_dir: None,
        };
        let shown = format!("{c:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("<secret>"), "{shown}");
    }

    #[test]
    fn values_skip_comments_and_blanks() {
        let text = "# EOS_PRODUCT_ID=no\nEOS_PRODUCT_ID = yes \nEOS_CLIENT_ID=\n";
        assert_eq!(value(text, "EOS_PRODUCT_ID"), Some("yes"));
        assert_eq!(value(text, "EOS_CLIENT_ID"), None);
        assert_eq!(value(text, "EOS_SANDBOX_ID"), None);
    }

    #[test]
    fn the_folders_in_order() {
        let exe = Path::new("/game/bin");
        let cwd = Path::new("/work");
        let p = places(Some(exe), Some(cwd), Some(PathBuf::from("/chosen")));
        assert_eq!(p[0], PathBuf::from("/chosen"));
        assert_eq!(p[1], PathBuf::from("/game/bin/eos"));
        assert_eq!(p[2], PathBuf::from("/game/bin/target/eos"));
        assert!(p.contains(&PathBuf::from("/game/target/eos")));
        assert!(p.contains(&PathBuf::from("/work/target/eos")));
    }

    #[test]
    fn wrong_files_say_what_is_wrong_without_the_secret() {
        let dir = std::env::temp_dir().join(format!("aa-eos-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let e = ConfigError::NoSecret { dir: dir.clone() }.to_string();
        assert!(e.contains(SECRET_FILE) && e.contains("AA_EOS_CLIENT_SECRET"), "{e}");
        let e = ConfigError::MissingId { file: dir.join(IDS_FILE), key: "EOS_PRODUCT_ID" }.to_string();
        assert!(e.contains("EOS_PRODUCT_ID"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

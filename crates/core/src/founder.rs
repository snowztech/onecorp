//! You, the founder: a name and an optional photo, kept on this Mac and shared by all your
//! companies. No account: this only tells the app and your employees who they work for.

use crate::store::home;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Founder {
    pub name: String,
    /// The photo's file inside ~/.onecorp, if you added one.
    pub photo: Option<PathBuf>,
}

fn file() -> PathBuf {
    home().join("founder.json")
}

pub fn load() -> Option<Founder> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file()).ok()?).ok()?;
    let name = v["name"].as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let photo = v["photo"].as_str().map(|p| home().join(p)).filter(|p| p.exists());
    Some(Founder { name, photo })
}

/// How prompts refer to you: by name once you've set one.
pub fn name() -> String {
    load().map(|f| f.name).unwrap_or_else(|| "the founder".into())
}

const PHOTO_TYPES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "heic"];

/// What to do with the photo when saving.
pub enum Photo<'a> {
    Keep,
    Remove,
    /// Copied in, so moving or deleting the original doesn't break it.
    Use(&'a Path),
}

/// Saves your profile. A new photo is checked and copied before the old one goes, so a bad
/// file never loses the photo you had.
pub fn save(name: &str, photo: Photo) -> Result<Founder> {
    let name = name.trim();
    if name.is_empty() {
        bail!("say what to call you, like Lucas");
    }
    fs::create_dir_all(home())?;
    let old = load().and_then(|f| f.photo);
    let kept = match photo {
        Photo::Keep => old,
        Photo::Remove => {
            if let Some(p) = old {
                let _ = fs::remove_file(p);
            }
            None
        }
        Photo::Use(src) => {
            let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !PHOTO_TYPES.contains(&ext.as_str()) {
                bail!("use a photo file: {}", PHOTO_TYPES.join(", "));
            }
            let size = fs::metadata(src).with_context(|| format!("couldn't read {}", src.display()))?.len();
            if size > 8 * 1024 * 1024 {
                bail!("that photo is over 8 MB. Pick a smaller one");
            }
            let dest = home().join(format!("founder-photo.{ext}"));
            fs::copy(src, &dest).with_context(|| format!("couldn't copy {}", src.display()))?;
            if let Some(p) = old.filter(|p| *p != dest) {
                let _ = fs::remove_file(p);
            }
            Some(dest)
        }
    };
    let photo_name = kept.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    fs::write(file(), serde_json::to_string_pretty(&json!({"name": name, "photo": photo_name}))?)?;
    Ok(Founder { name: name.to_string(), photo: kept })
}

/// Forgets you: your name and photo. The next launch asks again. Companies stay.
pub fn remove() -> Result<()> {
    if let Some(p) = load().and_then(|f| f.photo) {
        let _ = fs::remove_file(p);
    }
    match fs::remove_file(file()) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

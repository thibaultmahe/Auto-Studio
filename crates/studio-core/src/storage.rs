use crate::{error::invalid, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn id(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_err(|_| invalid("Identifiant photo invalide."))
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    // Fichier complet d’abord, publication sans écrasement par lien atomique.
    let parent = path.parent().ok_or_else(|| invalid("Dossier invalide."))?;
    let tmp = parent.join(format!(".{}.part", Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::hard_link(&tmp, path)?;
        Ok(())
    })();
    let _ = fs::remove_file(&tmp);
    result
}
pub fn retained(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        if hash(&fs::read(path)?) != hash(bytes) {
            return Err(invalid(
                "Un fichier local existe avec un contenu différent.",
            ));
        }
        return Ok(());
    }
    write_new(path, bytes)
}
pub fn safe_export(directory: &Path, stem: &str, extension: &str, bytes: &[u8]) -> Result<PathBuf> {
    if !directory.is_dir() {
        return Err(invalid("Choisissez un dossier d’export existant."));
    }
    let name: String = stem
        .chars()
        .take(64)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    for n in 1..=9999 {
        let path = directory.join(format!("autostudio-{name}-{n:04}.{extension}"));
        match write_new(&path, bytes) {
            Ok(()) => return Ok(path),
            Err(crate::Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(invalid(
        "Trop de fichiers portent déjà ce nom dans le dossier d’export.",
    ))
}

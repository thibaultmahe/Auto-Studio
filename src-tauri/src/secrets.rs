use zeroize::Zeroizing;
type Result<T> = std::result::Result<T, String>;

#[cfg(windows)]
fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new("fr.pujolauto.autostudio.inference-sh", "api-key")
        .map_err(|_| "Le gestionnaire d’identifiants Windows est indisponible.".into())
}
pub fn get() -> Result<Zeroizing<String>> {
    #[cfg(windows)]
    {
        entry()?
            .get_password()
            .map(Zeroizing::new)
            .map_err(|_| "Clé API absente ou gestionnaire d’identifiants inaccessible.".into())
    }
    #[cfg(not(windows))]
    {
        Err("La saisie et le stockage de la clé nécessitent Windows dans cette version.".into())
    }
}
pub fn present() -> Result<bool> {
    #[cfg(windows)]
    {
        match entry()?.get_password() {
            Ok(s) => {
                let _secret = Zeroizing::new(s);
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(_) => Err("Impossible de consulter le gestionnaire d’identifiants Windows.".into()),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}
pub fn save(value: String) -> Result<()> {
    let value = Zeroizing::new(value);
    if value.len() < 8 || value.len() > 512 || value.chars().any(char::is_whitespace) {
        return Err("Le format de la clé API est invalide.".into());
    }
    #[cfg(windows)]
    {
        entry()?
            .set_password(&value)
            .map_err(|_| "Impossible d’enregistrer la clé dans Windows.".into())
    }
    #[cfg(not(windows))]
    {
        Err("Le stockage sécurisé de la clé nécessite Windows dans cette version.".into())
    }
}
pub fn delete() -> Result<()> {
    #[cfg(windows)]
    {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("Impossible de supprimer la clé dans Windows.".into()),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

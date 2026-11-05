#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    User(String),
    #[error("Impossible de lire ou d’écrire les données locales.")]
    Io(#[from] std::io::Error),
    #[error("La base locale est indisponible.")]
    Db(#[from] rusqlite::Error),
    #[error("Le contenu de l’image est invalide ou non pris en charge.")]
    Image(#[from] image::ImageError),
    #[error("Les réglages enregistrés sont invalides.")]
    Json(#[from] serde_json::Error),
    // Ne jamais afficher l’erreur reqwest : elle peut contenir une URL signée.
    #[error("Connexion indisponible ou délai réseau dépassé.")]
    Network,
    #[error("{}", http_message(*.0))]
    Http(u16),
}
fn http_message(status: u16) -> &'static str {
    match status {
    401|403=>"La clé API est absente, invalide ou ne permet pas cette opération.",
    402=>"Le fournisseur refuse l’opération pour une raison de facturation ; vérifiez votre compte.",
    429=>"Limite de requêtes atteinte. Réessayez la consultation plus tard.",
    413=>"Le fournisseur refuse la taille de cette image.",
    500..=599=>"Le fournisseur est temporairement indisponible.",
    _=>"Le fournisseur a refusé la requête.",
}
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn invalid(message: &str) -> Error {
    Error::User(message.into())
}

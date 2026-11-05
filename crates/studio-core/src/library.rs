use crate::{
    compose, error::invalid, imaging, model::*, provider::SegmentationProvider, storage, Error,
    Result,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::DynamicImage;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Library {
    root: PathBuf,
    _lock: Arc<fs::File>,
}
impl Library {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        fs::create_dir_all(root.as_ref())?;
        let root = fs::canonicalize(root.as_ref())?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".library.lock"))?;
        lock.try_lock()
            .map_err(|_| invalid("Cette photothèque est déjà ouverte dans une autre instance."))?;
        let this = Self {
            root,
            _lock: Arc::new(lock),
        };
        for d in [
            "originals",
            "normalized",
            "masks",
            "cutouts",
            "backgrounds",
            "previews",
            "renders",
            "exports",
        ] {
            fs::create_dir_all(this.root.join(d))?;
        }
        let mut conn = this.connection()?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => {
                let tx = conn.transaction()?;
                tx.execute_batch(include_str!("../migrations/001.sql"))?;
                tx.commit()?;
            }
            1 => {}
            _ => {
                return Err(invalid(
                    "Cette photothèque nécessite une version plus récente d’AutoStudio.",
                ))
            }
        }
        conn.execute("UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error='Envoi interrompu avant soumission : vous pouvez confirmer un nouvel envoi.' WHERE id IN (SELECT photo_id FROM submissions WHERE state='UPLOADING')",[])?;
        conn.execute(
            "UPDATE submissions SET state='REJECTED' WHERE state='UPLOADING'",
            [],
        )?;
        conn.execute("UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error='Soumission interrompue : retrouvez son identifiant chez le fournisseur avant toute nouvelle tentative.' WHERE id IN (SELECT photo_id FROM submissions WHERE state='SUBMITTING')",[])?;
        conn.execute(
            "UPDATE submissions SET state='AMBIGUOUS' WHERE state='SUBMITTING'",
            [],
        )?;
        for (id, dark) in [("STUDIO_CLAIR", false), ("STUDIO_FONCE", true)] {
            let path = this.root.join("backgrounds").join(format!("{id}-v1.png"));
            if !path.exists() {
                let img = compose::background(1600, 1200, dark);
                let bytes = imaging::png(&DynamicImage::ImageRgba8(img))?;
                storage::retained(&path, &bytes)?;
                conn.execute("INSERT OR IGNORE INTO backgrounds VALUES (?1,1,?2,1600,1200,?3)",
                    params![id,storage::hash(&bytes),r#"{"safe_zone":[0.08,0.08,0.92,0.88],"ground":0.88,"generator":"gradient-v1"}"#])?;
            }
        }
        Ok(this)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn connection(&self) -> Result<Connection> {
        let c = Connection::open(self.root.join("studio.sqlite3"))?;
        c.busy_timeout(Duration::from_secs(5))?;
        c.pragma_update(None, "foreign_keys", true)?;
        Ok(c)
    }
    pub fn settings(&self) -> Result<Settings> {
        let value: Option<String> = self
            .connection()?
            .query_row("SELECT value FROM config WHERE key='settings'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(value
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .unwrap_or_default())
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        settings.limits.validate()?;
        if let Some(pin) = &settings.provider {
            pin.validate()?;
        }
        self.connection()?.execute("INSERT INTO config VALUES ('settings',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn pin_provider(&self, p: &dyn SegmentationProvider, key: &str) -> Result<ProviderPin> {
        let pin = p.describe(key)?;
        let mut settings = self.settings()?;
        settings.provider = Some(pin.clone());
        self.save_settings(&settings)?;
        Ok(pin)
    }
    pub fn import(&self, path: &Path) -> Result<Photo> {
        if !path.is_file() {
            return Err(invalid("Sélectionnez une photo JPEG, PNG ou WebP."));
        }
        let limits = self.settings()?.limits;
        if fs::metadata(path)?.len() > limits.bytes() {
            return Err(invalid(
                "Le fichier dépasse la limite de taille configurée.",
            ));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take(limits.bytes() + 1)
            .read_to_end(&mut bytes)?;
        let sha = storage::hash(&bytes);
        let conn = self.connection()?;
        let existing: Option<String> = conn
            .query_row("SELECT id FROM photos WHERE sha256=?1", [&sha], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(id) = existing {
            return self.photo(&id);
        }
        let decoded = imaging::decode(&bytes, &limits)?;
        let (w, h) = decoded.image.dimensions();
        let original = self
            .root
            .join("originals")
            .join(format!("{sha}.{}", decoded.extension));
        storage::retained(&original, &bytes)?;
        drop(bytes);
        let normalized = imaging::png(&DynamicImage::ImageRgba8(decoded.image))?;
        storage::retained(
            &self.root.join("normalized").join(format!("{sha}.png")),
            &normalized,
        )?;
        let preview = imaging::decode_stored(&normalized)?;
        drop(normalized);
        let thumbnail = DynamicImage::ImageRgba8(preview).thumbnail(512, 384);
        storage::retained(
            &self.root.join("previews").join(format!("{sha}.png")),
            &imaging::png(&thumbnail)?,
        )?;
        let id = Uuid::new_v4().to_string();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Photo")
            .chars()
            .take(180)
            .collect::<String>();
        conn.execute(
            "INSERT INTO photos(id,name,sha256,extension,width,height) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, name, sha, decoded.extension, w, h],
        )?;
        self.photo(&id)
    }
    pub fn photos(&self) -> Result<Vec<Photo>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare("SELECT id FROM photos ORDER BY created_at,id")?;
        let ids = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter().map(|id| self.photo(id)).collect()
    }
    pub fn photo(&self, id: &str) -> Result<Photo> {
        storage::id(id)?;
        let conn = self.connection()?;
        let value=conn.query_row("SELECT name,sha256,width,height,category,technical,review,mask_version,render_key,error,warnings,settings,mask_provider FROM photos WHERE id=?1",[id],|r| {
            Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u32>(2)?,r.get::<_,u32>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,u32>(7)?,r.get::<_,Option<String>>(8)?,r.get::<_,Option<String>>(9)?,r.get::<_,String>(10)?,r.get::<_,Option<String>>(11)?,r.get::<_,Option<String>>(12)?))
        }).optional()?.ok_or_else(||invalid("Photo introuvable."))?;
        let remote_id = conn
            .query_row(
                "SELECT remote_id FROM submissions WHERE photo_id=?1 ORDER BY rowid DESC LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        Ok(Photo {
            id: id.into(),
            name: value.0,
            sha256: value.1,
            width: value.2,
            height: value.3,
            category: serde_json::from_value(serde_json::Value::String(value.4))?,
            technical: value.5,
            review: value.6,
            mask_version: value.7,
            render_key: value.8,
            error: value.9,
            warnings: serde_json::from_str(&value.10)?,
            remote_id,
            settings: value.11.map(|s| serde_json::from_str(&s)).transpose()?,
            mask_provider: value.12.map(|s| serde_json::from_str(&s)).transpose()?,
        })
    }
    pub fn classify(&self, id: &str, category: Category) -> Result<Photo> {
        let p = self.photo(id)?;
        if ["SUBMITTING", "REMOTE_RUNNING"].contains(&p.technical.as_str()) {
            return Err(invalid(
                "Attendez la fin du détourage avant de changer la catégorie.",
            ));
        }
        if category != p.category {
            self.connection()?.execute("UPDATE photos SET category=?2,review='NEEDS_REVIEW',render_key=NULL,technical=?3 WHERE id=?1",params![id,serde_json::to_value(category)?.as_str(),if p.mask_version>0 {"MASK_READY"} else {"PENDING"}])?;
        }
        self.photo(id)
    }
    fn cache_key(&self, p: &Photo, pin: &ProviderPin) -> Result<String> {
        Ok(storage::hash(
            serde_json::to_string(&(
                "oriented-png-v1",
                &p.sha256,
                "inference.sh",
                "infsh/birefnet",
                pin,
            ))?
            .as_bytes(),
        ))
    }
    pub fn plan(&self, id: &str) -> Result<Plan> {
        let p = self.photo(id)?;
        if p.category != Category::Exterior {
            return Err(invalid(
                "Classez explicitement la photo en Extérieur avant tout détourage.",
            ));
        }
        let pin = self.settings()?.provider;
        let stored: Option<String> = self.connection()?.query_row(
            "SELECT mask_cache_key FROM photos WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        let cached = if let Some(pin) = &pin {
            p.mask_version > 0 && stored == Some(self.cache_key(&p, pin)?)
        } else {
            false
        };
        if !cached {
            if let Some(pin) = &pin {
                let key = self.cache_key(&p, pin)?;
                let existing:bool=self.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM submissions WHERE cache_key=?1 AND state!='REJECTED')",[key],|r|r.get(0))?;
                if existing {
                    return Err(invalid("Un traitement existe déjà. Utilisez Consulter / réconcilier ; aucun nouvel envoi n’est nécessaire."));
                }
            }
        }
        Ok(Plan{selected:1,calls:u32::from(!cached),cached:u32::from(cached),provider:"inference.sh".into(),model:"infsh/birefnet".into(),pin,
            cost_notice:"Tarif actuel non vérifié. Coût exact inconnu ; aucun montant fixe estimé. Le fournisseur peut facturer une soumission acceptée même en cas de délai réseau.".into()})
    }
    pub fn submit(
        &self,
        id: &str,
        expected_version: &str,
        provider: &dyn SegmentationProvider,
        key: &str,
    ) -> Result<Photo> {
        let plan = self.plan(id)?;
        let pin = plan
            .pin
            .ok_or_else(|| invalid("Vérifiez et épinglez le modèle dans les paramètres."))?;
        if pin.version_id != expected_version {
            return Err(invalid(
                "La version du modèle a changé. Relisez le récapitulatif avant confirmation.",
            ));
        }
        if plan.cached == 1 {
            return self.photo(id);
        }
        let photo = self.photo(id)?;
        let cache = self.cache_key(&photo, &pin)?;
        let mut conn = self.connection()?;
        // Claim transactionnel avant tout réseau : protège aussi contre un double clic.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing:Option<String>=tx.query_row("SELECT state FROM submissions WHERE cache_key=?1 AND state!='REJECTED' ORDER BY rowid DESC LIMIT 1",[&cache],|r|r.get(0)).optional()?;
        if existing.is_some() {
            return Err(invalid("Une soumission existe déjà. Consultez-la ou réconciliez son identifiant ; aucun nouvel appel facturable n’est envoyé."));
        }
        let job = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO submissions(id,photo_id,cache_key,app_id,version_id,state) VALUES (?1,?2,?3,?4,?5,'UPLOADING')",params![job,id,cache,pin.app_id,pin.version_id])?;
        tx.execute(
            "UPDATE photos SET technical='SUBMITTING',review='NEEDS_REVIEW',error=NULL WHERE id=?1",
            [id],
        )?;
        tx.commit()?;
        let source = fs::read(self.normalized(&photo)?)?;
        if source.len() as u64 > self.settings()?.limits.bytes() {
            self.reject_upload(
                id,
                &job,
                "Le PNG orienté dépasse la limite locale d’envoi. Aucun détourage n’a été soumis.",
            )?;
            return Err(invalid("Le PNG orienté est trop volumineux pour l’envoi."));
        }
        let uri = match provider.upload(key, source) {
            Ok(v) => v,
            Err(e) => {
                self.reject_upload(id, &job, &e.to_string())?;
                return Err(e);
            }
        };
        conn.execute(
            "UPDATE submissions SET remote_uri=?2,state='SUBMITTING' WHERE id=?1",
            params![job, uri],
        )?;
        // L’état SUBMITTING reste durable si le processus s’arrête pendant le POST.
        match provider.submit(key, &pin, &uri) {
            Ok(remote) => {
                conn.execute(
                    "UPDATE submissions SET remote_id=?2,state='REMOTE_RUNNING' WHERE id=?1",
                    params![job, remote],
                )?;
                conn.execute(
                    "UPDATE photos SET technical='REMOTE_RUNNING',error=NULL WHERE id=?1",
                    [id],
                )?;
            }
            Err(e) => {
                let rejected = matches!(
                    e,
                    Error::Http(400 | 401 | 402 | 403 | 404 | 413 | 422 | 429)
                );
                conn.execute(
                    "UPDATE submissions SET state=?2 WHERE id=?1",
                    params![job, if rejected { "REJECTED" } else { "AMBIGUOUS" }],
                )?;
                let message = if rejected {
                    e.to_string()
                } else {
                    "Soumission incertaine : vérifiez l’historique du fournisseur et rattachez son identifiant avant toute nouvelle tentative.".into()
                };
                conn.execute(
                    "UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error=?2 WHERE id=?1",
                    params![id, message],
                )?;
                return Err(invalid(&message));
            }
        }
        self.photo(id)
    }
    fn reject_upload(&self, id: &str, job: &str, message: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute("UPDATE submissions SET state='REJECTED' WHERE id=?1", [job])?;
        conn.execute(
            "UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error=?2 WHERE id=?1",
            params![id, message],
        )?;
        Ok(())
    }
    pub fn submissions(&self) -> Result<Vec<Submission>> {
        let conn = self.connection()?;
        let mut s=conn.prepare("SELECT id,photo_id,version_id,remote_id,state,created_at FROM submissions ORDER BY rowid DESC")?;
        let rows = s.query_map([], |r| {
            Ok(Submission {
                id: r.get(0)?,
                photo_id: r.get(1)?,
                version_id: r.get(2)?,
                remote_id: r.get(3)?,
                state: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    pub fn synchronize(
        &self,
        id: &str,
        attach: Option<&str>,
        provider: &dyn SegmentationProvider,
        key: &str,
    ) -> Result<Photo> {
        let photo = self.photo(id)?;
        let conn = self.connection()?;
        let (job,remote,app,version,uri,cache):(String,Option<String>,String,String,Option<String>,String)=conn.query_row(
            "SELECT id,remote_id,app_id,version_id,remote_uri,cache_key FROM submissions WHERE photo_id=?1 ORDER BY rowid DESC LIMIT 1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
        let remote=match (remote,attach) {
            (Some(r),Some(a)) if r!=a=>return Err(invalid("Un autre identifiant est déjà rattaché à cette photo.")),
            (Some(r),_)=>r, (None,Some(a))=>a.into(),
            _=>return Err(invalid("Retrouvez le traitement dans votre compte fournisseur puis renseignez son identifiant.")),
        };
        let task = provider.task(key, &remote)?;
        if task.app_id != app
            || task.app_version_id != version
            || task.input["image"].as_str() != uri.as_deref()
            || uri.is_none()
        {
            return Err(invalid(
                "Ce traitement distant ne correspond pas à cette source et à sa version de modèle.",
            ));
        }
        conn.execute(
            "UPDATE submissions SET remote_id=?2,state='REMOTE_RUNNING' WHERE id=?1",
            params![job, remote],
        )?;
        match task.status {
            10 => {
                let stored: Option<String> =
                    conn.query_row("SELECT mask_cache_key FROM photos WHERE id=?1", [id], |r| {
                        r.get(0)
                    })?;
                if photo.mask_version > 0 && stored.as_deref() == Some(&cache) {
                    conn.execute(
                        "UPDATE submissions SET state='COMPLETED' WHERE id=?1",
                        [&job],
                    )?;
                    return self.photo(id);
                }
                let bytes = provider.result(key, &remote, self.settings()?.limits.bytes())?;
                let source = imaging::decode_stored(&fs::read(self.normalized(&photo)?)?)?;
                let (mask, cutout, warnings) = match imaging::remote_mask(
                    &bytes,
                    &source,
                    &self.settings()?.limits,
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        conn.execute(
                                "UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error=?2 WHERE id=?1",
                                params![id, e.to_string()],
                            )?;
                        return Err(e);
                    }
                };
                let v = photo.mask_version + 1;
                storage::retained(
                    &self.root.join("masks").join(format!("{id}-v{v}.png")),
                    &imaging::png(&DynamicImage::ImageLuma8(mask))?,
                )?;
                storage::retained(
                    &self.root.join("cutouts").join(format!("{id}-v{v}.png")),
                    &imaging::png(&DynamicImage::ImageRgba8(cutout))?,
                )?;
                let pin = ProviderPin {
                    app_id: app,
                    version_id: version,
                    function: None,
                };
                conn.execute("UPDATE photos SET mask_version=?2,mask_cache_key=?3,mask_provider=?4,technical='MASK_READY',review='NEEDS_REVIEW',render_key=NULL,error=NULL,warnings=?5 WHERE id=?1",
                    params![id,v,cache,serde_json::to_string(&pin)?,serde_json::to_string(&warnings)?])?;
                conn.execute(
                    "UPDATE submissions SET state='COMPLETED' WHERE id=?1",
                    [job],
                )?;
            }
            11..=13 => {
                conn.execute("UPDATE submissions SET state='FAILED' WHERE id=?1", [job])?;
                conn.execute("UPDATE photos SET technical='FAILED',review='NEEDS_REVIEW',error='Le traitement distant a échoué, a été annulé ou refusé. Consultez votre compte fournisseur.' WHERE id=?1",[id])?;
            }
            _ => {
                conn.execute(
                    "UPDATE photos SET technical='REMOTE_RUNNING',error=NULL WHERE id=?1",
                    [id],
                )?;
            }
        }
        self.photo(id)
    }
    pub fn compose(&self, id: &str, settings: &Composition) -> Result<Photo> {
        settings.validate()?;
        let p = self.photo(id)?;
        if p.category != Category::Exterior {
            return Err(invalid(
                "Seuls les extérieurs explicitement classés peuvent être composés.",
            ));
        }
        if ["SUBMITTING", "REMOTE_RUNNING"].contains(&p.technical.as_str()) {
            return Err(invalid(
                "Attendez la fin du détourage en cours avant de recomposer.",
            ));
        }
        if p.mask_version == 0 {
            return Err(invalid(
                "Un masque valide est nécessaire pour composer un extérieur.",
            ));
        }
        let cutout = imaging::decode_stored(&fs::read(self.asset_path(&p, Asset::Cutout)?)?)?;
        let mask = DynamicImage::ImageRgba8(imaging::decode_stored(&fs::read(
            self.asset_path(&p, Asset::Mask)?,
        )?)?)
        .to_luma8();
        let key = storage::hash(
            serde_json::to_string(&(
                "renderer-v1",
                &p.sha256,
                p.mask_version,
                storage::hash(mask.as_raw()),
                compose::BACKGROUND_VERSION,
                settings,
            ))?
            .as_bytes(),
        );
        let path = self.root.join("renders").join(format!("{key}.png"));
        if !path.exists() {
            let result = compose::render(&cutout, &mask, settings)?;
            storage::retained(&path, &imaging::png(&DynamicImage::ImageRgba8(result))?)?;
        }
        if p.render_key.as_deref() != Some(&key) {
            self.connection()?.execute("UPDATE photos SET settings=?2,render_key=?3,technical='COMPLETED',review='NEEDS_REVIEW',error=NULL WHERE id=?1",params![id,serde_json::to_string(settings)?,key])?;
        }
        self.photo(id)
    }
    pub fn review(&self, id: &str, approved: bool) -> Result<Photo> {
        let p = self.photo(id)?;
        if approved && (p.render_key.is_none() || p.technical != "COMPLETED") {
            return Err(invalid(
                "Composez et vérifiez le résultat avant de l’approuver.",
            ));
        }
        self.connection()?.execute(
            "UPDATE photos SET review=?2 WHERE id=?1",
            params![id, if approved { "APPROVED" } else { "REJECTED" }],
        )?;
        self.photo(id)
    }
    fn normalized(&self, p: &Photo) -> Result<PathBuf> {
        if p.sha256.len() != 64 || !p.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("Empreinte locale invalide."));
        }
        Ok(self
            .root
            .join("normalized")
            .join(format!("{}.png", p.sha256)))
    }
    fn asset_path(&self, p: &Photo, kind: Asset) -> Result<PathBuf> {
        self.normalized(p)?;
        storage::id(&p.id)?;
        let path = match kind {
            Asset::Original => self.normalized(p)?,
            Asset::Mask | Asset::Cutout => {
                if p.mask_version == 0 {
                    return Err(invalid("Le détourage n’est pas encore disponible."));
                }
                let d = if matches!(kind, Asset::Mask) {
                    "masks"
                } else {
                    "cutouts"
                };
                self.root
                    .join(d)
                    .join(format!("{}-v{}.png", p.id, p.mask_version))
            }
            Asset::Render => {
                let k = p
                    .render_key
                    .as_ref()
                    .ok_or_else(|| invalid("Composez d’abord un rendu."))?;
                if k.len() != 64 || !k.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(invalid("Empreinte du rendu invalide."));
                }
                self.root.join("renders").join(format!("{k}.png"))
            }
        };
        let real = fs::canonicalize(&path)?;
        if !real.starts_with(&self.root) {
            return Err(invalid("Le fichier sort de la photothèque."));
        }
        Ok(real)
    }
    pub fn preview(&self, id: &str, kind: Asset, thumbnail: bool) -> Result<String> {
        let p = self.photo(id)?;
        let path = if thumbnail && matches!(kind, Asset::Original) {
            self.root.join("previews").join(format!("{}.png", p.sha256))
        } else {
            self.asset_path(&p, kind)?
        };
        let image = imaging::decode_stored(&fs::read(path)?)?;
        let size = if thumbnail { 512 } else { 1600 };
        let image = DynamicImage::ImageRgba8(image).thumbnail(size, size);
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(imaging::png(&image)?)
        ))
    }
    pub fn export(
        &self,
        id: &str,
        directory: &Path,
        format: ExportFormat,
        quality: u8,
    ) -> Result<PathBuf> {
        if !(1..=100).contains(&quality) {
            return Err(invalid("La qualité doit être comprise entre 1 et 100."));
        }
        let p = self.photo(id)?;
        if !matches!(format, ExportFormat::Original)
            && (p.review != "APPROVED" || p.technical != "COMPLETED")
        {
            return Err(invalid(
                "Approuvez ce résultat avant d’exporter une image traitée.",
            ));
        }
        let (extension, bytes) = match format {
            ExportFormat::Original => {
                let ext: String = self.connection()?.query_row(
                    "SELECT extension FROM photos WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )?;
                if !["jpg", "png", "webp"].contains(&ext.as_str()) {
                    return Err(invalid("Format original invalide."));
                }
                let bytes = fs::read(
                    self.root
                        .join("originals")
                        .join(format!("{}.{}", p.sha256, ext)),
                )?;
                if storage::hash(&bytes) != p.sha256 {
                    return Err(invalid("L’original ne correspond plus à son empreinte."));
                }
                (ext, bytes)
            }
            ExportFormat::Mask => ("png".into(), fs::read(self.asset_path(&p, Asset::Mask)?)?),
            ExportFormat::Cutout => ("png".into(), fs::read(self.asset_path(&p, Asset::Cutout)?)?),
            ExportFormat::Jpeg | ExportFormat::Webp => {
                let img = imaging::decode_stored(&fs::read(self.asset_path(&p, Asset::Render)?)?)?;
                if matches!(format, ExportFormat::Jpeg) {
                    let mut bytes = Vec::new();
                    let rgb = DynamicImage::ImageRgba8(img).to_rgb8();
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
                        .encode_image(&DynamicImage::ImageRgb8(rgb))?;
                    ("jpg".into(), bytes)
                } else {
                    let bytes = webp::Encoder::from_rgba(img.as_raw(), img.width(), img.height())
                        .encode(quality as f32)
                        .to_vec();
                    ("webp".into(), bytes)
                }
            }
        };
        storage::safe_export(
            directory,
            &format!("{}-{}", p.name, &p.id[..8]),
            &extension,
            &bytes,
        )
    }
}

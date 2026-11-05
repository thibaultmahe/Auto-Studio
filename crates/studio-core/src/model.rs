use crate::{error::invalid, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Unknown,
    Exterior,
    Interior,
    Detail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Limits {
    pub max_file_mib: u32,
    pub max_megapixels: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_mib: 32,
            max_megapixels: 24,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<()> {
        if !(1..=64).contains(&self.max_file_mib) || !(1..=40).contains(&self.max_megapixels) {
            return Err(invalid(
                "Limites autorisées : 1 à 64 Mio et 1 à 40 mégapixels.",
            ));
        }
        Ok(())
    }
    pub fn bytes(&self) -> u64 {
        u64::from(self.max_file_mib) * 1024 * 1024
    }
    pub fn pixels(&self) -> u64 {
        u64::from(self.max_megapixels) * 1_000_000
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderPin {
    pub app_id: String,
    pub version_id: String,
    #[serde(default)]
    pub function: Option<String>,
}
impl ProviderPin {
    pub fn validate(&self) -> Result<()> {
        valid_remote_id(&self.app_id)?;
        valid_remote_id(&self.version_id)?;
        if let Some(f) = &self.function {
            valid_remote_id(f)?;
        }
        Ok(())
    }
}
pub fn valid_remote_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("Identifiant distant invalide."));
    }
    Ok(())
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub limits: Limits,
    pub provider: Option<ProviderPin>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub name: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub category: Category,
    pub technical: String,
    pub review: String,
    pub mask_version: u32,
    pub render_key: Option<String>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub remote_id: Option<String>,
    pub settings: Option<Composition>,
    pub mask_provider: Option<ProviderPin>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Composition {
    pub width: u32,
    pub height: u32,
    pub background: String,
    pub margin: f32,
    pub scale: f32,
    pub horizontal: f32,
    pub ground: f32,
    pub shadow: bool,
    pub shadow_opacity: f32,
    pub shadow_blur: f32,
    pub shadow_offset: f32,
}
impl Default for Composition {
    fn default() -> Self {
        Self {
            width: 1600,
            height: 1200,
            background: "STUDIO_CLAIR".into(),
            margin: 0.08,
            scale: 1.0,
            horizontal: 0.5,
            ground: 0.88,
            shadow: true,
            shadow_opacity: 0.16,
            shadow_blur: 0.035,
            shadow_offset: 0.0,
        }
    }
}
impl Composition {
    pub fn validate(&self) -> Result<()> {
        if self.width < 256 || self.height < 256 || self.width > 4096 || self.height > 4096 {
            return Err(invalid(
                "Les dimensions de sortie doivent être comprises entre 256 et 4096 pixels.",
            ));
        }
        for (value, lo, hi) in [
            (self.margin, 0.02, 0.25),
            (self.scale, 0.1, 1.0),
            (self.horizontal, 0.0, 1.0),
            (self.ground, 0.55, 0.95),
            (self.shadow_opacity, 0.0, 0.4),
            (self.shadow_blur, 0.005, 0.12),
            (self.shadow_offset, -0.1, 0.1),
        ] {
            if !value.is_finite() || value < lo || value > hi {
                return Err(invalid("Réglages de cadrage invalides."));
            }
        }
        if !["STUDIO_CLAIR", "STUDIO_FONCE"].contains(&self.background.as_str()) {
            return Err(invalid("Fond indisponible dans la phase 1."));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Asset {
    Original,
    Cutout,
    Mask,
    Render,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Original,
    Mask,
    Cutout,
    Webp,
    Jpeg,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub selected: u32,
    pub calls: u32,
    pub cached: u32,
    pub provider: String,
    pub model: String,
    pub pin: Option<ProviderPin>,
    pub cost_notice: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub photo_id: String,
    pub version_id: String,
    pub remote_id: Option<String>,
    pub state: String,
    pub created_at: String,
}

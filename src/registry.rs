//! Registre de modules fun — index JSON partagé.
//!
//! Permet de :
//! - lister les modules disponibles dans un registre (local ou distant) ;
//! - publier un module (métadonnées + binaire + signature) dans un registre ;
//! - installer un module depuis un registre.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Entrée d'un module dans le registre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntry {
    /// Nom du module (unique).
    pub name: String,
    /// Version semver.
    pub version: String,
    /// Description courte.
    pub description: String,
    /// Auteur(s).
    #[serde(default)]
    pub authors: Vec<String>,
    /// Licence.
    #[serde(default = "default_license")]
    pub license: String,
    /// Catégorie (game, tool, visual, network, other…).
    #[serde(default = "default_category")]
    pub category: String,
    /// URL de téléchargement du binaire (.so/.dylib/.dll).
    pub download_url: String,
    /// URL de la signature Ed25519 (fichier .sig).
    #[serde(default)]
    pub signature_url: Option<String>,
    /// Hash SHA-256 du binaire (hex).
    #[serde(default)]
    pub sha256: Option<String>,
    /// Cible de compilation (ex: x86_64-unknown-linux-gnu).
    #[serde(default)]
    pub target: Option<String>,
    /// Page du projet / dépôt source.
    #[serde(default)]
    pub homepage: Option<String>,
    /// Date de publication (RFC3339).
    #[serde(default)]
    pub published_at: Option<String>,
}

fn default_license() -> String {
    "MIT".to_string()
}

fn default_category() -> String {
    "other".to_string()
}

/// Registre complet (index JSON).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    /// Nom du registre.
    #[serde(default = "default_registry_name")]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Version du format d'index.
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// Modules disponibles.
    #[serde(default)]
    pub modules: Vec<ModuleEntry>,
}

fn default_registry_name() -> String {
    "zer0fun registry".to_string()
}

fn default_schema() -> u32 {
    1
}

impl Registry {
    /// Charge un registre depuis un fichier local ou une URL http(s).
    pub async fn load(source: &str) -> Result<Self> {
        if source.starts_with("http://") || source.starts_with("https://") {
            let resp = reqwest::get(source).await
                .with_context(|| format!("échec de téléchargement du registre: {}", source))?;
            if !resp.status().is_success() {
                return Err(anyhow!("registre inaccessible (HTTP {}) : {}", resp.status(), source));
            }
            let text = resp.text().await?;
            let reg: Registry = serde_json::from_str(&text)
                .with_context(|| "JSON de registre invalide")?;
            Ok(reg)
        } else {
            let text = std::fs::read_to_string(source)
                .with_context(|| format!("registre introuvable: {}", source))?;
            let reg: Registry = serde_json::from_str(&text)
                .with_context(|| "JSON de registre invalide")?;
            Ok(reg)
        }
    }

    /// Sauvegarde le registre dans un fichier local.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Ajoute ou met à jour une entrée (par nom).
    pub fn upsert(&mut self, entry: ModuleEntry) {
        if let Some(existing) = self.modules.iter_mut().find(|m| m.name == entry.name) {
            *existing = entry;
        } else {
            self.modules.push(entry);
        }
        self.modules.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Supprime une entrée par nom.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.modules.len();
        self.modules.retain(|m| m.name != name);
        self.modules.len() != before
    }

    /// Cherche une entrée par nom.
    pub fn find(&self, name: &str) -> Option<&ModuleEntry> {
        self.modules.iter().find(|m| m.name == name)
    }
}

/// Chemin par défaut du registre local.
pub fn default_registry_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Roaming".to_string())))
            .join("zer0build")
            .join("registry.json")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("zer0build")
            .join("registry.json")
    }
}

/// Liste les modules d'un registre (local ou URL).
pub async fn list(source: &str) -> Result<()> {
    let reg = Registry::load(source).await?;
    println!("Registre : {} ({})", reg.name, source);
    if !reg.description.is_empty() {
        println!("{}", reg.description);
    }
    println!();
    if reg.modules.is_empty() {
        println!("  (aucun module)");
        return Ok(());
    }
    println!("{:<20} {:<10} {:<12} {}", "NOM", "VERSION", "CATÉGORIE", "DESCRIPTION");
    println!("{}", "-".repeat(80));
    for m in &reg.modules {
        println!("{:<20} {:<10} {:<12} {}", m.name, m.version, m.category, m.description);
    }
    println!();
    println!("{} module(s) disponible(s).", reg.modules.len());
    Ok(())
}

/// Publie un module dans le registre local.
///
/// Lit `zer0fun.toml` du projet courant, calcule le SHA-256 du binaire,
/// et ajoute/met à jour l'entrée dans le registre.
pub async fn publish(
    binary: &Path,
    registry_path: Option<PathBuf>,
    download_url: Option<String>,
    homepage: Option<String>,
) -> Result<()> {
    let reg_path = registry_path.unwrap_or_else(default_registry_path);

    // Lire les métadonnées du module
    let meta_path = Path::new("zer0fun.toml");
    if !meta_path.exists() {
        return Err(anyhow!("zer0fun.toml introuvable — lancez cette commande depuis le dossier du module"));
    }
    let meta_text = std::fs::read_to_string(meta_path)?;
    let meta: toml::Value = toml::from_str(&meta_text)?;
    let module = meta.get("module").ok_or_else(|| anyhow!("section [module] absente de zer0fun.toml"))?;

    let name = module.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("champ 'name' manquant"))?.to_string();
    let version = module.get("version").and_then(|v| v.as_str())
        .unwrap_or("0.1.0").to_string();
    let description = module.get("description").and_then(|v| v.as_str())
        .unwrap_or("").to_string();
    let authors: Vec<String> = module.get("authors")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let license = module.get("license").and_then(|v| v.as_str())
        .unwrap_or("MIT").to_string();
    let category = module.get("category").and_then(|v| v.as_str())
        .unwrap_or("other").to_string();

    // Hash SHA-256 du binaire
    let sha256 = compute_sha256(binary)?;

    // URL de téléchargement (par défaut : chemin local file://)
    let url = download_url.unwrap_or_else(|| {
        format!("file://{}", binary.canonicalize().unwrap_or_else(|_| binary.to_path_buf()).display())
    });

    let entry = ModuleEntry {
        name: name.clone(),
        version,
        description,
        authors,
        license,
        category,
        download_url: url,
        signature_url: None,
        sha256: Some(sha256),
        target: None,
        homepage,
        published_at: Some(now_rfc3339()),
    };

    let mut reg = if reg_path.exists() {
        Registry::load(reg_path.to_str().unwrap_or("")).await.unwrap_or_else(|_| Registry {
            name: "zer0fun registry".to_string(),
            description: "Registre local de modules fun zer0gh0st".to_string(),
            schema: 1,
            modules: Vec::new(),
        })
    } else {
        Registry {
            name: "zer0fun registry".to_string(),
            description: "Registre local de modules fun zer0gh0st".to_string(),
            schema: 1,
            modules: Vec::new(),
        }
    };
    reg.upsert(entry);
    reg.save(&reg_path)?;

    println!("✅ Module '{}' publié dans {}", name, reg_path.display());
    Ok(())
}

/// Installe un module depuis un registre (local ou URL).
pub async fn install_from_registry(
    name: &str,
    source: &str,
    target_dir: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    let reg = Registry::load(source).await?;
    let entry = reg.find(name)
        .ok_or_else(|| anyhow!("module '{}' introuvable dans le registre", name))?;

    let install_dir = target_dir
        .or_else(|| crate::config::Config::load().ok().and_then(|c| c.install_dir))
        .unwrap_or_else(crate::compiler::default_install_dir_public);
    std::fs::create_dir_all(&install_dir)?;

    let dest = install_dir.join(format!("{}.so", name));
    if dest.exists() && !force {
        return Err(anyhow!("module '{}' déjà installé (utiliser --force)", name));
    }

    // Télécharger ou copier le binaire
    if entry.download_url.starts_with("http://") || entry.download_url.starts_with("https://") {
        let bytes = reqwest::get(&entry.download_url).await?
            .error_for_status()?
            .bytes().await?;
        std::fs::write(&dest, &bytes)?;
    } else {
        let src = entry.download_url.strip_prefix("file://").unwrap_or(&entry.download_url);
        std::fs::copy(src, &dest)
            .with_context(|| format!("copie impossible depuis {}", src))?;
    }

    // Vérifier le hash si présent
    if let Some(expected) = &entry.sha256 {
        let actual = compute_sha256(&dest)?;
        if &actual != expected {
            let _ = std::fs::remove_file(&dest);
            return Err(anyhow!("hash SHA-256 invalide pour '{}' (attendu {}, obtenu {})", name, expected, actual));
        }
        println!("🔒 Hash SHA-256 vérifié");
    }

    println!("✅ Module '{}' installé dans {}", name, dest.display());
    Ok(())
}

/// Calcule le SHA-256 d'un fichier (hex).
fn compute_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

/// Retire un module d'un registre local.
pub async fn remove_from_registry(name: &str, registry_path: Option<PathBuf>) -> Result<()> {
    let reg_path = registry_path.unwrap_or_else(default_registry_path);
    if !reg_path.exists() {
        return Err(anyhow!("registre introuvable: {}", reg_path.display()));
    }
    let mut reg = Registry::load(reg_path.to_str().unwrap_or("")).await?;
    if reg.remove(name) {
        reg.save(&reg_path)?;
        println!("✅ Module '{}' retiré du registre {}", name, reg_path.display());
        Ok(())
    } else {
        Err(anyhow!("module '{}' introuvable dans le registre", name))
    }
}

/// Date RFC3339 approximative (sans dépendance chrono).
fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // Format simple : epoch (les consommateurs peuvent parser)
    format!("{}", secs)
}

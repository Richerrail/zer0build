//! Compilation et installation des modules fun.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Cibles de compilation supportées.
pub const TARGETS: &[(&str, &str)] = &[
    ("x86_64-unknown-linux-gnu", "linux-x64"),
    ("x86_64-unknown-linux-musl", "linux-musl"),
    ("aarch64-unknown-linux-gnu", "linux-arm64"),
    ("x86_64-apple-darwin", "macos-x64"),
    ("aarch64-apple-darwin", "macos-arm64"),
    ("x86_64-pc-windows-msvc", "windows-x64"),
];

/// Compile le module courant.
pub async fn build(target: Option<String>, all: bool, release: bool) -> Result<()> {
    let manifest = std::fs::read_to_string("Cargo.toml")?;
    let package_name = extract_package_name(&manifest)?;
    
    println!("🔨 Compilation de {}...", package_name);
    
    if all {
        // Cross-compilation pour toutes les cibles
        for (triple, name) in TARGETS {
            println!("  → {} ({})", name, triple);
            build_target(triple, release).await?;
        }
    } else if let Some(t) = target {
        // Cible spécifique
        build_target(&t, release).await?;
    } else {
        // Cible native
        build_native(release).await?;
    }
    
    println!("✅ Compilation terminée");
    Ok(())
}

/// Compile pour la cible native.
async fn build_native(release: bool) -> Result<()> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    
    let status = cmd.status()?;
    if !status.success() {
        return Err(anyhow!("échec de la compilation"));
    }
    Ok(())
}

/// Compile pour une cible spécifique.
async fn build_target(triple: &str, release: bool) -> Result<()> {
    // Vérifier si la cible est installée
    let installed = check_target_installed(triple).await?;
    if !installed {
        println!("    Installation de la cible {}...", triple);
        install_target(triple).await?;
    }
    
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build")
        .arg("--target")
        .arg(triple);
    if release {
        cmd.arg("--release");
    }
    
    let status = cmd.status()?;
    if !status.success() {
        return Err(anyhow!("échec de la compilation pour {}", triple));
    }
    Ok(())
}

/// Vérifie si une cible Rust est installée.
async fn check_target_installed(triple: &str) -> Result<bool> {
    let output = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line.trim() == triple))
}

/// Installe une cible Rust.
async fn install_target(triple: &str) -> Result<()> {
    let status = std::process::Command::new("rustup")
        .args(["target", "add", triple])
        .status()?;
    
    if !status.success() {
        return Err(anyhow!("échec de l'installation de la cible {}", triple));
    }
    Ok(())
}

/// Installe le module compilé dans zer0gh0st.
pub async fn install(
    path: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    // Trouver le .so
    let so_path = match path {
        Some(p) => p,
        None => find_compiled_module()?,
    };
    
    if !so_path.exists() {
        return Err(anyhow!("fichier non trouvé: {}", so_path.display()));
    }
    
    // Déterminer le répertoire d'installation
    let install_dir = target_dir.unwrap_or_else(default_install_dir);
    std::fs::create_dir_all(&install_dir)?;
    
    // Nom du module
    let module_name = so_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .trim_start_matches("lib")
        .to_string();
    
    let dest = install_dir.join(format!("{}.so", module_name));
    
    // Vérifier si déjà installé
    if dest.exists() && !force {
        return Err(anyhow!(
            "module '{}' déjà installé. Utiliser --force pour écraser.",
            module_name
        ));
    }
    
    // Copier
    std::fs::copy(&so_path, &dest)?;
    println!("✅ Module '{}' installé dans {}", module_name, dest.display());
    
    // Signer le module
    crate::signer::sign_module(&dest, None).await?;
    
    Ok(())
}

/// Liste les modules installés.
pub async fn list_installed(target_dir: Option<PathBuf>) -> Result<()> {
    let install_dir = target_dir.unwrap_or_else(default_install_dir);
    
    if !install_dir.exists() {
        println!("Aucun module installé (répertoire {} inexistant)", install_dir.display());
        return Ok(());
    }
    
    println!("Modules installés dans {}:", install_dir.display());
    println!();
    
    let mut entries: Vec<_> = std::fs::read_dir(&install_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|s| s == "so").unwrap_or(false))
        .collect();
    
    entries.sort_by_key(|e| e.file_name());
    
    if entries.is_empty() {
        println!("  (aucun)");
    } else {
        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let size = entry.metadata()?.len();
            let modified = entry.metadata()?.modified()?;
            
            println!("  {} ({} octets, modifié {:?})", name, size, modified);
        }
    }
    
    Ok(())
}

/// Supprime un module installé.
pub async fn remove(name: &str, target_dir: Option<PathBuf>) -> Result<()> {
    let install_dir = target_dir.unwrap_or_else(default_install_dir);
    let path = install_dir.join(format!("{}.so", name));
    
    if !path.exists() {
        return Err(anyhow!("module '{}' non trouvé", name));
    }
    
    std::fs::remove_file(&path)?;
    
    // Supprimer aussi la signature si présente
    let sig_path = install_dir.join(format!("{}.so.sig", name));
    if sig_path.exists() {
        std::fs::remove_file(&sig_path)?;
    }
    
    println!("✅ Module '{}' supprimé", name);
    Ok(())
}

/// Trouve le module compilé dans target/.
fn find_compiled_module() -> Result<PathBuf> {
    // Chercher dans target/release puis target/debug
    for profile in ["release", "debug"] {
        let dir = Path::new("target").join(profile);
        if !dir.exists() {
            continue;
        }
        
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Chercher les .so (Linux), .dll (Windows), .dylib (macOS)
            let is_lib = path.extension().map(|e| {
                e == "so" || e == "dll" || e == "dylib"
            }).unwrap_or(false);
            
            if is_lib && path.file_name().map(|n| {
                n.to_string_lossy().starts_with("lib")
            }).unwrap_or(false) {
                return Ok(path);
            }
        }
    }
    
    Err(anyhow!("aucun module compilé trouvé. Exécuter 'zer0build build' d'abord."))
}

/// Répertoire d'installation par défaut.
pub fn default_install_dir_public() -> PathBuf {
    default_install_dir()
}

fn default_install_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Roaming".to_string())))
            .join("zer0gh0st")
            .join("fun")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("zer0gh0st")
            .join("fun")
    }
}

/// Extrait le nom du package depuis Cargo.toml.
fn extract_package_name(manifest: &str) -> Result<String> {
    let parsed: toml::Value = toml::from_str(manifest)?;
    parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("nom de package non trouvé dans Cargo.toml"))
}

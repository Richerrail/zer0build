//! Signature et vérification des modules fun.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};

/// Signe un module compilé.
pub async fn sign_module(path: &Path, key_path: Option<PathBuf>) -> Result<()> {
    // Charger ou générer la clé privée
    let signing_key = match key_path {
        Some(p) => load_private_key(&p)?,
        None => get_or_create_keypair()?.0,
    };
    
    // Calculer le hash du fichier
    let hash = compute_file_hash(path)?;
    
    // Signer
    let signature = signing_key.sign(&hash);
    
    // Sauvegarder la signature
    let sig_path = path.with_extension("so.sig");
    std::fs::write(&sig_path, signature.to_bytes())?;
    
    println!("✅ Module signé: {}", sig_path.display());
    
    // Sauvegarder la clé publique si première utilisation
    let pubkey_path = default_pubkey_path();
    if !pubkey_path.exists() {
        let verifying_key = signing_key.verifying_key();
        std::fs::write(&pubkey_path, verifying_key.to_bytes())?;
        println!("🔑 Clé publique sauvegardée: {}", pubkey_path.display());
    }
    
    Ok(())
}

/// Vérifie la signature d'un module.
pub async fn verify_module(path: &Path, pubkey_path: Option<PathBuf>) -> Result<()> {
    // Charger la clé publique
    let pubkey_path = pubkey_path.unwrap_or_else(default_pubkey_path);
    let verifying_key = load_public_key(&pubkey_path)?;
    
    // Charger la signature
    let sig_path = path.with_extension("so.sig");
    if !sig_path.exists() {
        return Err(anyhow!("signature non trouvée: {}", sig_path.display()));
    }
    let sig_bytes = std::fs::read(&sig_path)?;
    let signature = Signature::from_bytes(&sig_bytes.try_into().map_err(|_| anyhow!("signature invalide"))?);
    
    // Calculer le hash du fichier
    let hash = compute_file_hash(path)?;
    
    // Vérifier
    verifying_key.verify(&hash, &signature)
        .map_err(|_| anyhow!("signature invalide pour {}", path.display()))?;
    
    println!("✅ Signature valide pour {}", path.display());
    Ok(())
}

/// Calcule le hash SHA-256 d'un fichier.
fn compute_file_hash(path: &Path) -> Result<Vec<u8>> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hasher.finalize().to_vec())
}

/// Charge une clé privée depuis un fichier.
fn load_private_key(path: &Path) -> Result<SigningKey> {
    let bytes = std::fs::read(path)?;
    let bytes: [u8; 32] = bytes.try_into()
        .map_err(|_| anyhow!("clé privée invalide (doit faire 32 octets)"))?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Charge une clé publique depuis un fichier.
fn load_public_key(path: &Path) -> Result<VerifyingKey> {
    let bytes = std::fs::read(path)?;
    let bytes: [u8; 32] = bytes.try_into()
        .map_err(|_| anyhow!("clé publique invalide (doit faire 32 octets)"))?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|e| anyhow!("clé publique invalide: {}", e))
}

/// Retourne ou crée une paire de clés.
fn get_or_create_keypair() -> Result<(SigningKey, VerifyingKey)> {
    let key_path = default_key_path();
    
    if key_path.exists() {
        let signing_key = load_private_key(&key_path)?;
        let verifying_key = signing_key.verifying_key();
        Ok((signing_key, verifying_key))
    } else {
        // Générer une nouvelle paire
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        
        // Sauvegarder la clé privée
        std::fs::create_dir_all(key_path.parent().unwrap())?;
        std::fs::write(&key_path, signing_key.to_bytes())?;
        
        // Permissions restrictives
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&key_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&key_path, perms)?;
        }
        
        println!("🔑 Nouvelle paire de clés générée: {}", key_path.display());
        
        Ok((signing_key, verifying_key))
    }
}

/// Chemin par défaut de la clé privée.
fn default_key_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Roaming".to_string())))
            .join("zer0build")
            .join("signing_key")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("zer0build")
            .join("signing_key")
    }
}

/// Chemin par défaut de la clé publique.
fn default_pubkey_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Roaming".to_string())))
            .join("zer0build")
            .join("verify_key.pub")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("zer0build")
            .join("verify_key.pub")
    }
}

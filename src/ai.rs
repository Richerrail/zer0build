//! Génération de modules via IA (OpenRouter).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Client OpenRouter.
pub struct OpenRouterClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
        };
        
        let response = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("erreur OpenRouter: {}", error));
        }
        
        let chat_response: ChatResponse = response.json().await?;
        
        chat_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("réponse OpenRouter vide"))
    }
}

/// Génère un module complet via IA.
pub async fn generate_module(
    prompt: &str,
    name: Option<String>,
    model: &str,
    api_key: Option<String>,
) -> Result<()> {
    // Récupérer la clé API : argument > variable d'env > fichier de config
    let cfg = crate::config::Config::load().unwrap_or_default();
    let api_key = api_key
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
        .or(cfg.openrouter_api_key)
        .ok_or_else(|| anyhow!(
            "clé API OpenRouter requise. Définir OPENROUTER_API_KEY, utiliser --api-key, ou renseigner openrouter_api_key dans ~/.config/zer0build/config.toml"
        ))?;
    
    let client = OpenRouterClient::new(api_key, model.to_string());
    
    // Générer le nom si non fourni
    let name = match name {
        Some(n) => n,
        None => generate_name(&client, prompt).await?,
    };
    
    println!("🤖 Génération du module '{}' via {}...", name, model);
    
    // Générer le code
    let system_prompt = include_str!("../prompts/module_system.md");
    let user_prompt = format!(
        "Crée un module fun zer0gh0st nommé '{}' avec cette description:\n\n{}",
        name, prompt
    );
    
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        Message {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];
    
    let response = client.chat(messages).await?;
    
    // Parser la réponse et créer les fichiers
    parse_and_create_module(&name, &response).await?;
    
    println!("✅ Module '{}' généré", name);
    println!();
    println!("Prochaines étapes:");
    println!("  cd {}", name);
    println!("  zer0build build --release");
    println!("  zer0build install");
    
    Ok(())
}

/// Génère un nom de module à partir du prompt.
async fn generate_name(client: &OpenRouterClient, prompt: &str) -> Result<String> {
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "Tu es un assistant qui génère des noms de modules. Réponds UNIQUEMENT avec le nom, en minuscules, sans espaces, max 20 caractères.".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: format!("Génère un nom pour ce module: {}", prompt),
        },
    ];
    
    let name = client.chat(messages).await?;
    let name = name.trim().to_lowercase();
    
    // Valider
    if name.is_empty() || name.len() > 32 {
        return Err(anyhow!("nom généré invalide: {}", name));
    }
    
    Ok(name)
}

/// Parse la réponse IA et crée les fichiers du module.
async fn parse_and_create_module(name: &str, response: &str) -> Result<()> {
    let project_dir = std::path::Path::new(name);
    
    if project_dir.exists() {
        return Err(anyhow!("le répertoire '{}' existe déjà", name));
    }
    
    std::fs::create_dir_all(project_dir.join("src"))?;
    
    // Extraire les fichiers de la réponse
    // Format attendu: ```toml:Cargo.toml ... ``` et ```rust:src/lib.rs ... ```
    
    let mut cargo_toml = None;
    let mut lib_rs = None;
    let mut zer0fun_toml = None;
    
    for block in extract_code_blocks(response) {
        let (lang, filename, content) = block;
        
        match (lang.as_str(), filename.as_str()) {
            ("toml", "Cargo.toml") => cargo_toml = Some(content),
            ("rust", "src/lib.rs") => lib_rs = Some(content),
            ("toml", "zer0fun.toml") => zer0fun_toml = Some(content),
            _ => {}
        }
    }
    
    // Écrire les fichiers
    if let Some(content) = cargo_toml {
        std::fs::write(project_dir.join("Cargo.toml"), content)?;
    } else {
        return Err(anyhow!("Cargo.toml non trouvé dans la réponse IA"));
    }
    
    if let Some(content) = lib_rs {
        std::fs::write(project_dir.join("src/lib.rs"), content)?;
    } else {
        return Err(anyhow!("src/lib.rs non trouvé dans la réponse IA"));
    }
    
    if let Some(content) = zer0fun_toml {
        std::fs::write(project_dir.join("zer0fun.toml"), content)?;
    }
    
    Ok(())
}

/// Extrait les blocs de code de la réponse.
fn extract_code_blocks(response: &str) -> Vec<(String, String, String)> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lang = String::new();
    let mut current_filename = String::new();
    let mut current_content = String::new();
    
    for line in response.lines() {
        if line.starts_with("```") {
            if in_block {
                // Fin du bloc
                blocks.push((
                    current_lang.clone(),
                    current_filename.clone(),
                    current_content.trim().to_string(),
                ));
                in_block = false;
                current_lang.clear();
                current_filename.clear();
                current_content.clear();
            } else {
                // Début du bloc
                let header = line.trim_start_matches("```");
                if let Some((lang, filename)) = header.split_once(':') {
                    current_lang = lang.to_string();
                    current_filename = filename.to_string();
                } else {
                    current_lang = header.to_string();
                    current_filename = String::new();
                }
                in_block = true;
            }
        } else if in_block {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    
    blocks
}

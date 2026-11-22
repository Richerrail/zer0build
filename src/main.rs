//! zer0build — CLI pour construire des modules fun zer0gh0st.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod scaffold;
mod compiler;
mod signer;
mod ai;
mod config;

use anyhow::Result;

#[derive(Parser)]
#[command(name = "zer0build")]
#[command(about = "Construit des modules fun pour zer0gh0st", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Répertoire de travail (défaut: courant)
    #[arg(short, long, global = true)]
    workdir: Option<PathBuf>,
    
    /// Verbosité
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Crée un nouveau projet de module
    New {
        /// Nom du module (minuscules, chiffres, -, _)
        name: String,
        
        /// Template à utiliser
        #[arg(short, long, default_value = "basic")]
        template: String,
        
        /// Description du module
        #[arg(short, long)]
        description: Option<String>,
        
        /// Auteur
        #[arg(short, long)]
        author: Option<String>,
    },
    
    /// Compile le module courant
    Build {
        /// Cible de compilation (ex: x86_64-unknown-linux-gnu)
        #[arg(short, long)]
        target: Option<String>,
        
        /// Compiler pour toutes les cibles zer0gh0st
        #[arg(long)]
        all: bool,
        
        /// Mode release
        #[arg(long)]
        release: bool,
    },
    
    /// Installe le module dans zer0gh0st
    Install {
        /// Chemin du .so à installer (défaut: target/release/lib<name>.so)
        path: Option<PathBuf>,
        
        /// Répertoire d'installation (défaut: ~/.local/share/zer0gh0st/fun/)
        #[arg(short, long)]
        target_dir: Option<PathBuf>,
        
        /// Forcer l'écrasement
        #[arg(short, long)]
        force: bool,
    },
    
    /// Liste les modules installés
    List {
        /// Répertoire d'installation
        #[arg(short, long)]
        target_dir: Option<PathBuf>,
    },
    
    /// Supprime un module installé
    Remove {
        /// Nom du module
        name: String,
        
        /// Répertoire d'installation
        #[arg(short, long)]
        target_dir: Option<PathBuf>,
    },
    
    /// Génère un module via IA (OpenRouter)
    Ai {
        /// Description du module à générer
        prompt: String,
        
        /// Nom du module (défaut: généré par l'IA)
        #[arg(short, long)]
        name: Option<String>,
        
        /// Modèle OpenRouter à utiliser
        #[arg(short, long, default_value = "anthropic/claude-3.5-sonnet")]
        model: String,
        
        /// Clé API OpenRouter (ou variable OPENROUTER_API_KEY)
        #[arg(short, long)]
        api_key: Option<String>,
    },
    
    /// Signe un module compilé
    Sign {
        /// Chemin du .so à signer
        path: PathBuf,
        
        /// Clé privée de signature (défaut: générée si absente)
        #[arg(short, long)]
        key: Option<PathBuf>,
    },
    
    /// Vérifie la signature d'un module
    Verify {
        /// Chemin du .so signé
        path: PathBuf,
        
        /// Clé publique de vérification
        #[arg(short, long)]
        pubkey: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialiser le logging
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
    
    // Changer de répertoire si demandé
    if let Some(dir) = &cli.workdir {
        std::env::set_current_dir(dir)?;
    }
    
    match cli.command {
        Commands::New { name, template, description, author } => {
            scaffold::create_project(&name, &template, description, author).await?;
        }
        Commands::Build { target, all, release } => {
            compiler::build(target, all, release).await?;
        }
        Commands::Install { path, target_dir, force } => {
            compiler::install(path, target_dir, force).await?;
        }
        Commands::List { target_dir } => {
            compiler::list_installed(target_dir).await?;
        }
        Commands::Remove { name, target_dir } => {
            compiler::remove(&name, target_dir).await?;
        }
        Commands::Ai { prompt, name, model, api_key } => {
            ai::generate_module(&prompt, name, &model, api_key).await?;
        }
        Commands::Sign { path, key } => {
            signer::sign_module(&path, key).await?;
        }
        Commands::Verify { path, pubkey } => {
            signer::verify_module(&path, pubkey).await?;
        }
    }
    
    Ok(())
}

//! Génération de templates de modules fun.

use anyhow::{anyhow, Result};
use std::path::Path;

/// Templates disponibles
pub const TEMPLATES: &[(&str, &str)] = &[
    ("basic", "Module minimal avec compteur"),
    ("game", "Jeu avec boucle et événements"),
    ("visual", "Visualisation/animation"),
    ("tool", "Outil/utilitaire"),
    ("network", "Module avec accès réseau"),
];

/// Crée un nouveau projet de module.
pub async fn create_project(
    name: &str,
    template: &str,
    description: Option<String>,
    author: Option<String>,
) -> Result<()> {
    // Valider le nom
    validate_name(name)?;
    
    // Vérifier que le template existe
    if !TEMPLATES.iter().any(|(t, _)| *t == template) {
        return Err(anyhow!(
            "template '{}' inconnu. Disponibles: {}",
            template,
            TEMPLATES.iter().map(|(t, _)| *t).collect::<Vec<_>>().join(", ")
        ));
    }
    
    // Vérifier que le répertoire n'existe pas
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(anyhow!("le répertoire '{}' existe déjà", name));
    }
    
    println!("📦 Création du module '{}' (template: {})", name, template);
    
    // Créer la structure
    std::fs::create_dir_all(project_dir.join("src"))?;
    
    // Générer les fichiers
    let description = description.unwrap_or_else(|| format!("Module fun {}", name));
    let author = author.unwrap_or_else(|| "Anonymous".to_string());
    
    generate_cargo_toml(project_dir, name, &description, &author)?;
    generate_lib_rs(project_dir, name, template, &description)?;
    generate_zer0fun_toml(project_dir, name, &description, &author)?;
    
    println!("✅ Module créé dans ./{}/", name);
    println!();
    println!("Prochaines étapes:");
    println!("  cd {}", name);
    println!("  zer0build build --release");
    println!("  zer0build install");
    
    Ok(())
}

/// Valide un nom de module.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("le nom ne peut pas être vide"));
    }
    if name.len() > 32 {
        return Err(anyhow!("le nom est trop long (max 32 caractères)"));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return Err(anyhow!(
            "nom invalide: '{}' (doit être ^[a-z0-9_-]+$)",
            name
        ));
    }
    Ok(())
}

/// Génère le Cargo.toml du module.
fn generate_cargo_toml(dir: &Path, name: &str, description: &str, author: &str) -> Result<()> {
    let content = format!(r#"[package]
name = "zer0fun-{}"
version = "0.1.0"
edition = "2021"
description = "{}"
authors = ["{}"]

[lib]
crate-type = ["cdylib"]

[dependencies]
zer0fun-api = {{ path = "../zer0fun-api" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
"#, name, description, author);
    
    std::fs::write(dir.join("Cargo.toml"), content)?;
    Ok(())
}

/// Génère le lib.rs du module selon le template.
fn generate_lib_rs(dir: &Path, name: &str, template: &str, description: &str) -> Result<()> {
    let struct_name = to_pascal_case(name);
    
    let content = match template {
        "basic" => generate_basic_template(&struct_name, description),
        "game" => generate_game_template(&struct_name, description),
        "visual" => generate_visual_template(&struct_name, description),
        "tool" => generate_tool_template(&struct_name, description),
        "network" => generate_network_template(&struct_name, description),
        _ => return Err(anyhow!("template inconnu: {}", template)),
    };
    
    std::fs::write(dir.join("src/lib.rs"), content)?;
    Ok(())
}

/// Génère le zer0fun.toml (métadonnées du module).
fn generate_zer0fun_toml(dir: &Path, name: &str, description: &str, author: &str) -> Result<()> {
    let content = format!(r#"# Métadonnées du module fun zer0gh0st
# Ce fichier est lu par zer0build et zer0gh0st

[module]
name = "{}"
version = "0.1.0"
description = "{}"
authors = ["{}"]
license = "MIT"
category = "other"

# Permissions requises par le module
[permissions]
network = false
fs_read = false
fs_write = false
process = false
clipboard = false
notifications = false
audio = false

# Configuration par défaut du module
[config]
# exemple = "valeur"
"#, name, description, author);
    
    std::fs::write(dir.join("zer0fun.toml"), content)?;
    Ok(())
}

/// Convertit en PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split(|c| c == '_' || c == '-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Template basique.
fn generate_basic_template(struct_name: &str, description: &str) -> String {
    format!(r#"//! {} — {}

use zer0fun_api::{{FunModule, Context, Event, Meta, Rect, Buffer, KeyEvent, Category, Permissions}};

pub struct {} {{
    counter: u32,
}}

impl FunModule for {} {{
    fn meta() -> Meta {{
        Meta::new("{}", "0.1.0", "{}")
            .category(Category::Other)
            .permissions(Permissions::none())
    }}
    
    fn new(_ctx: Context) -> Self {{
        Self {{ counter: 0 }}
    }}
    
    fn on_event(&mut self, ev: Event, ctx: &mut Context) -> bool {{
        match ev {{
            Event::Key(KeyEvent::Char('+')) => {{
                self.counter += 1;
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Char('-')) => {{
                self.counter = self.counter.saturating_sub(1);
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Esc) => {{
                ctx.quit();
                true
            }}
            _ => false,
        }}
    }}
    
    fn draw(&self, area: Rect, buf: &mut Buffer) {{
        let text = format!("Compteur: {{}}", self.counter);
        let x = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
        let y = area.y + area.height / 2;
        buf.set_string(x, y, &text);
        
        let help = "[+/-] modifier  [Esc] quitter";
        let hx = area.x + (area.width.saturating_sub(help.len() as u16)) / 2;
        buf.set_string(hx, y + 2, help);
    }}
}}

zer0fun_api::register!({});
"#, struct_name, description, struct_name, struct_name, struct_name.to_lowercase(), description, struct_name)
}

/// Template jeu.
fn generate_game_template(struct_name: &str, description: &str) -> String {
    format!(r#"//! {} — {}

use zer0fun_api::{{FunModule, Context, Event, Meta, Rect, Buffer, KeyEvent, Category, Permissions}};

pub struct {} {{
    player_x: u16,
    player_y: u16,
    score: u32,
    game_over: bool,
}}

impl FunModule for {} {{
    fn meta() -> Meta {{
        Meta::new("{}", "0.1.0", "{}")
            .category(Category::Game)
            .permissions(Permissions::none())
    }}
    
    fn new(ctx: Context) -> Self {{
        Self {{
            player_x: ctx.term_width / 2,
            player_y: ctx.term_height / 2,
            score: 0,
            game_over: false,
        }}
    }}
    
    fn on_event(&mut self, ev: Event, ctx: &mut Context) -> bool {{
        if self.game_over {{
            if matches!(ev, Event::Key(KeyEvent::Char('r'))) {{
                *self = Self::new(ctx.clone());
                ctx.request_redraw();
            }}
            if matches!(ev, Event::Key(KeyEvent::Esc)) {{
                ctx.quit();
            }}
            return true;
        }}
        
        match ev {{
            Event::Key(KeyEvent::Up) => {{
                self.player_y = self.player_y.saturating_sub(1);
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Down) => {{
                self.player_y = (self.player_y + 1).min(ctx.term_height - 1);
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Left) => {{
                self.player_x = self.player_x.saturating_sub(1);
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Right) => {{
                self.player_x = (self.player_x + 1).min(ctx.term_width - 1);
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Esc) => {{
                ctx.quit();
                true
            }}
            _ => false,
        }}
    }}
    
    fn on_tick(&mut self, ctx: &mut Context) {{
        // Logique de jeu ici
        if !self.game_over {{
            self.score += 1;
            ctx.request_redraw();
        }}
    }}
    
    fn draw(&self, area: Rect, buf: &mut Buffer) {{
        // Bordure
        buf.draw_border(zer0fun_api::BorderStyle::Single);
        
        // Joueur
        buf.set_char(self.player_x, self.player_y, '@');
        
        // Score
        let score_text = format!("Score: {{}}", self.score);
        buf.set_string(area.x + 2, area.y, &score_text);
        
        if self.game_over {{
            let msg = "GAME OVER - [r] recommencer";
            let x = area.x + (area.width.saturating_sub(msg.len() as u16)) / 2;
            let y = area.y + area.height / 2;
            buf.set_string(x, y, msg);
        }}
    }}
}}

zer0fun_api::register!({});
"#, struct_name, description, struct_name, struct_name, struct_name.to_lowercase(), description, struct_name)
}

/// Template visualisation.
fn generate_visual_template(struct_name: &str, description: &str) -> String {
    format!(r#"//! {} — {}

use zer0fun_api::{{FunModule, Context, Event, Meta, Rect, Buffer, KeyEvent, Category, Permissions, Color, Style}};

pub struct {} {{
    frame: u32,
}}

impl FunModule for {} {{
    fn meta() -> Meta {{
        Meta::new("{}", "0.1.0", "{}")
            .category(Category::Visual)
            .permissions(Permissions::none())
    }}
    
    fn new(_ctx: Context) -> Self {{
        Self {{ frame: 0 }}
    }}
    
    fn on_event(&mut self, ev: Event, ctx: &mut Context) -> bool {{
        match ev {{
            Event::Key(KeyEvent::Esc) => {{
                ctx.quit();
                true
            }}
            _ => false,
        }}
    }}
    
    fn on_tick(&mut self, ctx: &mut Context) {{
        self.frame += 1;
        ctx.request_redraw();
    }}
    
    fn draw(&self, area: Rect, buf: &mut Buffer) {{
        // Animation simple: vague
        for x in 0..area.width {{
            let y = area.y + area.height / 2 
                + ((x as f32 * 0.3 + self.frame as f32 * 0.1).sin() * 3.0) as u16;
            if y < area.height {{
                let style = Style {{
                    fg: Some(Color::Cyan),
                    ..Default::default()
                }};
                buf.set_string_styled(area.x + x, y, "~", style);
            }}
        }}
        
        let info = format!("Frame: {{}}", self.frame);
        buf.set_string(area.x + 1, area.y, &info);
    }}
}}

zer0fun_api::register!({});
"#, struct_name, description, struct_name, struct_name, struct_name.to_lowercase(), description, struct_name)
}

/// Template outil.
fn generate_tool_template(struct_name: &str, description: &str) -> String {
    format!(r#"//! {} — {}

use zer0fun_api::{{FunModule, Context, Event, Meta, Rect, Buffer, KeyEvent, Category, Permissions}};

pub struct {} {{
    input: String,
    output: String,
}}

impl FunModule for {} {{
    fn meta() -> Meta {{
        Meta::new("{}", "0.1.0", "{}")
            .category(Category::Tool)
            .permissions(Permissions::none())
    }}
    
    fn new(_ctx: Context) -> Self {{
        Self {{
            input: String::new(),
            output: "Entrez du texte...".to_string(),
        }}
    }}
    
    fn on_event(&mut self, ev: Event, ctx: &mut Context) -> bool {{
        match ev {{
            Event::Key(KeyEvent::Char(c)) => {{
                self.input.push(c);
                self.process();
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Backspace) => {{
                self.input.pop();
                self.process();
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Enter) => {{
                self.input.clear();
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Esc) => {{
                ctx.quit();
                true
            }}
            _ => false,
        }}
    }}
    
    fn draw(&self, area: Rect, buf: &mut Buffer) {{
        buf.draw_border(zer0fun_api::BorderStyle::Rounded);
        
        let title = " Outil ";
        buf.set_string(area.x + 2, area.y, title);
        
        // Input
        let input_label = "Entrée: ";
        buf.set_string(area.x + 2, area.y + 2, input_label);
        buf.set_string(area.x + 2 + input_label.len() as u16, area.y + 2, &self.input);
        
        // Output
        let output_label = "Sortie: ";
        buf.set_string(area.x + 2, area.y + 4, output_label);
        buf.set_string(area.x + 2 + output_label.len() as u16, area.y + 4, &self.output);
        
        // Help
        let help = "[Entrée] effacer  [Esc] quitter";
        let hx = area.x + (area.width.saturating_sub(help.len() as u16)) / 2;
        buf.set_string(hx, area.y + area.height - 1, help);
    }}
}}

impl {} {{
    fn process(&mut self) {{
        // Traitement ici
        self.output = format!("{{}} caractères", self.input.len());
    }}
}}

zer0fun_api::register!({});
"#, struct_name, description, struct_name, struct_name, struct_name.to_lowercase(), description, struct_name, struct_name)
}

/// Template réseau.
fn generate_network_template(struct_name: &str, description: &str) -> String {
    format!(r#"//! {} — {}

use zer0fun_api::{{FunModule, Context, Event, Meta, Rect, Buffer, KeyEvent, Category, Permissions}};

pub struct {} {{
    status: String,
    data: String,
}}

impl FunModule for {} {{
    fn meta() -> Meta {{
        Meta::new("{}", "0.1.0", "{}")
            .category(Category::Network)
            .permissions(Permissions::network_only())
    }}
    
    fn new(_ctx: Context) -> Self {{
        Self {{
            status: "Prêt".to_string(),
            data: String::new(),
        }}
    }}
    
    fn on_mount(&mut self, ctx: &mut Context) {{
        self.status = "Connexion...".to_string();
        ctx.request_redraw();
        
        // Simuler une requête réseau
        // En réalité, utiliser reqwest ou autre
        self.status = "Connecté".to_string();
        self.data = "Données reçues".to_string();
    }}
    
    fn on_event(&mut self, ev: Event, ctx: &mut Context) -> bool {{
        match ev {{
            Event::Key(KeyEvent::Char('r')) => {{
                self.status = "Rafraîchissement...".to_string();
                ctx.request_redraw();
                true
            }}
            Event::Key(KeyEvent::Esc) => {{
                ctx.quit();
                true
            }}
            _ => false,
        }}
    }}
    
    fn draw(&self, area: Rect, buf: &mut Buffer) {{
        buf.draw_border(zer0fun_api::BorderStyle::Double);
        
        let title = " Réseau ";
        buf.set_string(area.x + 2, area.y, title);
        
        buf.set_string(area.x + 2, area.y + 2, &format!("Statut: {{}}", self.status));
        buf.set_string(area.x + 2, area.y + 4, &format!("Données: {{}}", self.data));
        
        let help = "[r] rafraîchir  [Esc] quitter";
        let hx = area.x + (area.width.saturating_sub(help.len() as u16)) / 2;
        buf.set_string(hx, area.y + area.height - 1, help);
    }}
}}

zer0fun_api::register!({});
"#, struct_name, description, struct_name, struct_name, struct_name.to_lowercase(), description, struct_name)
}

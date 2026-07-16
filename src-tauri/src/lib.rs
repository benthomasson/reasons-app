use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    image::Image,
    Manager,
};

struct ServerState {
    running: bool,
    port: u16,
    default_db_path: PathBuf,
    cancel_token: CancellationToken,
}

struct AppState {
    server: Arc<Mutex<ServerState>>,
    install_item: tauri::menu::MenuItem<tauri::Wry>,
    install_code_item: Option<tauri::menu::MenuItem<tauri::Wry>>,
}

fn claude_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    { Some(dirs::home_dir()?.join("Library/Application Support/Claude/claude_desktop_config.json")) }
    #[cfg(target_os = "windows")]
    { Some(dirs::config_dir()?.join("Claude/claude_desktop_config.json")) }
    #[cfg(target_os = "linux")]
    { Some(dirs::config_dir()?.join("Claude/claude_desktop_config.json")) }
}

fn is_installed_in_claude() -> bool {
    let Some(path) = claude_config_path() else { return false };
    let Ok(content) = std::fs::read_to_string(&path) else { return false };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) else { return false };
    config.get("mcpServers")
        .and_then(|s| s.get("reasons"))
        .is_some()
}

fn reasons_binary_path() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("reasons")
}

fn has_claude_code() -> bool {
    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_installed_in_claude_code() -> bool {
    std::process::Command::new("claude")
        .args(["mcp", "get", "reasons"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn install_to_claude_code(db_path: &PathBuf) -> Result<(), String> {
    let binary = reasons_binary_path();
    let binary_str = binary.to_string_lossy().to_string();
    let db_str = db_path.to_string_lossy().to_string();

    let output = std::process::Command::new("claude")
        .args(["mcp", "add", "--scope", "user", "reasons", "--", &binary_str, "mcp", "--db", &db_str])
        .output()
        .map_err(|e| format!("Failed to run claude: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("claude mcp add failed: {}", stderr))
    }
}

fn install_to_claude(db_path: &PathBuf) -> Result<(), String> {
    let config_path = claude_config_path()
        .ok_or("Could not find home directory")?;

    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        serde_json::json!({})
    };

    let binary = reasons_binary_path();
    let binary_str = binary.to_string_lossy().to_string();
    let db_str = db_path.to_string_lossy().to_string();

    let servers = config.as_object_mut()
        .ok_or("Config is not an object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    servers.as_object_mut()
        .ok_or("mcpServers is not an object")?
        .insert("reasons".to_string(), serde_json::json!({
            "command": binary_str,
            "args": ["mcp", "--db", db_str]
        }));

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

async fn start_mcp_server(domain_list: Vec<(String, PathBuf)>, default_domain: String, port: u16, cancel_token: CancellationToken) {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    for (name, path) in &domain_list {
        if !path.exists() {
            eprintln!("Initializing database for domain '{}': {}", name, path.display());
            if let Err(e) = reasons_core::db::init_db(path) {
                eprintln!("Failed to initialize database for domain '{}': {}", name, e);
                return;
            }
        }
    }

    if let Err(e) = reasons_core::mcp::run_http_server(domain_list, default_domain, addr, cancel_token).await {
        eprintln!("MCP server error: {}", e);
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            reasons_core::config::ensure_default_config();
            let (domain_list, default_domain) = reasons_core::config::load_domains();
            let port: u16 = 6519;
            let cancel_token = CancellationToken::new();
            let status_item = MenuItemBuilder::new(format!("Status: running on port {}", port))
                .id("status")
                .enabled(false)
                .build(app)?;

            let domains_label = if domain_list.len() == 1 {
                format!("Database: {}", domain_list[0].1.display())
            } else {
                format!("Domains: {} configured (default: {})", domain_list.len(), default_domain)
            };
            let db_item = MenuItemBuilder::new(domains_label)
                .id("db_path")
                .enabled(false)
                .build(app)?;
            let installed = is_installed_in_claude();
            let install_label = if installed {
                "Claude Desktop: Installed ✓"
            } else {
                "Install in Claude Desktop"
            };
            let install_item = MenuItemBuilder::new(install_label)
                .id("install_claude")
                .enabled(!installed)
                .build(app)?;

            let install_code_item = if has_claude_code() {
                let installed_code = is_installed_in_claude_code();
                let code_label = if installed_code {
                    "Claude Code: Installed ✓"
                } else {
                    "Install in Claude Code"
                };
                Some(MenuItemBuilder::new(code_label)
                    .id("install_claude_code")
                    .enabled(!installed_code)
                    .build(app)?)
            } else {
                None
            };

            let quit_item = MenuItemBuilder::new("Quit")
                .id("quit")
                .build(app)?;

            let mut menu = MenuBuilder::new(app)
                .item(&status_item)
                .item(&db_item)
                .separator()
                .item(&install_item);
            if let Some(ref code_item) = install_code_item {
                menu = menu.item(code_item);
            }
            let menu = menu
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::with_id("reasons-tray")
                .icon(Image::from_bytes(include_bytes!("../icons/icon.png"))?)
                .icon_as_template(true)
                .tooltip("Reasons MCP Server")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app_handle, event| {
                    match event.id().as_ref() {
                        "quit" => {
                            let state = app_handle.state::<AppState>();
                            let server = state.server.clone();
                            let app = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let s = server.lock().await;
                                s.cancel_token.cancel();
                                drop(s);
                                app.exit(0);
                            });
                        }
                        "install_claude" => {
                            let state = app_handle.state::<AppState>();
                            let server = state.server.clone();
                            let app = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let s = server.lock().await;
                                let db_path = s.default_db_path.clone();
                                drop(s);
                                match install_to_claude(&db_path) {
                                    Ok(()) => {
                                        eprintln!("Installed reasons MCP server in Claude Desktop config");
                                        let state = app.state::<AppState>();
                                        let _ = state.install_item.set_text("Claude Desktop: Installed ✓");
                                        let _ = state.install_item.set_enabled(false);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to install: {}", e);
                                    }
                                }
                            });
                        }
                        "install_claude_code" => {
                            let state = app_handle.state::<AppState>();
                            let server = state.server.clone();
                            let app = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let s = server.lock().await;
                                let db_path = s.default_db_path.clone();
                                drop(s);
                                match install_to_claude_code(&db_path) {
                                    Ok(()) => {
                                        eprintln!("Installed reasons MCP server in Claude Code");
                                        let state = app.state::<AppState>();
                                        if let Some(ref item) = state.install_code_item {
                                            let _ = item.set_text("Claude Code: Installed ✓");
                                            let _ = item.set_enabled(false);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to install in Claude Code: {}", e);
                                    }
                                }
                            });
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let default_db = domain_list.iter()
                .find(|(name, _)| name == &default_domain)
                .map(|(_, path)| path.clone())
                .unwrap_or_else(|| domain_list[0].1.clone());

            let state = AppState {
                server: Arc::new(Mutex::new(ServerState {
                    running: true,
                    port,
                    default_db_path: default_db,
                    cancel_token: cancel_token.clone(),
                })),
                install_item: install_item.clone(),
                install_code_item: install_code_item.clone(),
            };
            app.manage(state);

            let default_clone = default_domain.clone();
            tauri::async_runtime::spawn(async move {
                start_mcp_server(domain_list, default_clone, port, cancel_token).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Reasons app");
}

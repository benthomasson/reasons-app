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

const CLAUDE_CONFIG_PATH: &str = "Library/Application Support/Claude/claude_desktop_config.json";

struct ServerState {
    running: bool,
    port: u16,
    db_path: PathBuf,
    cancel_token: CancellationToken,
}

struct AppState {
    server: Arc<Mutex<ServerState>>,
    install_item: tauri::menu::MenuItem<tauri::Wry>,
}

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasons.db")
}

fn claude_config_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(CLAUDE_CONFIG_PATH))
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

async fn start_mcp_server(db_path: PathBuf, port: u16, cancel_token: CancellationToken) {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    if !db_path.exists() {
        if let Err(e) = reasons_core::db::init_db(&db_path) {
            eprintln!("Failed to initialize database: {}", e);
            return;
        }
    }

    if let Err(e) = reasons_core::mcp::run_http_server(db_path, addr, cancel_token).await {
        eprintln!("MCP server error: {}", e);
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let db_path = default_db_path();
            let port: u16 = 6519;
            let cancel_token = CancellationToken::new();
            let status_item = MenuItemBuilder::new(format!("Status: running on port {}", port))
                .id("status")
                .enabled(false)
                .build(app)?;
            let db_item = MenuItemBuilder::new(format!("Database: {}", db_path.display()))
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
            let quit_item = MenuItemBuilder::new("Quit")
                .id("quit")
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&status_item)
                .item(&db_item)
                .separator()
                .item(&install_item)
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
                                let db_path = s.db_path.clone();
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
                        _ => {}
                    }
                })
                .build(app)?;

            let state = AppState {
                server: Arc::new(Mutex::new(ServerState {
                    running: true,
                    port,
                    db_path: db_path.clone(),
                    cancel_token: cancel_token.clone(),
                })),
                install_item: install_item.clone(),
            };
            app.manage(state);

            tauri::async_runtime::spawn(async move {
                start_mcp_server(db_path, port, cancel_token).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Reasons app");
}

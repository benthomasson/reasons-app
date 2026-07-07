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
    db_path: PathBuf,
    cancel_token: CancellationToken,
}

struct AppState {
    server: Arc<Mutex<ServerState>>,
}

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasons.db")
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

            let status_item = MenuItemBuilder::new(format!("Status: Running on port {}", port))
                .id("status")
                .enabled(false)
                .build(app)?;
            let db_item = MenuItemBuilder::new(format!("Database: {}", db_path.display()))
                .id("db_path")
                .enabled(false)
                .build(app)?;
            let quit_item = MenuItemBuilder::new("Quit")
                .id("quit")
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&status_item)
                .item(&db_item)
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
                    if event.id().as_ref() == "quit" {
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
                })
                .build(app)?;

            let state = AppState {
                server: Arc::new(Mutex::new(ServerState {
                    running: true,
                    port,
                    db_path: db_path.clone(),
                    cancel_token: cancel_token.clone(),
                })),
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

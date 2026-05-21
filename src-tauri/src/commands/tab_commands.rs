use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{
    async_runtime::JoinHandle, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime,
    WebviewUrl, Window,
};
use url::Url;

pub const TAB_BAR_HEIGHT: f64 = 50.0;
pub const MAIN_WINDOW_LABEL: &str = "main";
const SITE_PREFIX: &str = "site-";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub expected_hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub tabs: Vec<TabConfig>,
    pub keep_alive_interval_secs: u64,
}

pub struct AppState {
    pub keep_alive_handle: Mutex<Option<JoinHandle<()>>>,
    pub active_tab_id: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            keep_alive_handle: Mutex::new(None),
            active_tab_id: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    load_config_sync()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    save_config_sync(&config)
}

#[tauri::command]
pub fn create_site_webview(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;

    create_site_webview_on_window(&window, &id, &url)?;
    if let Some(webview) = app.get_webview(&site_label(&id)) {
        let _ = webview.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn show_site_webview(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    let next_label = site_label(&id);
    let next = app
        .get_webview(&next_label)
        .ok_or_else(|| format!("webview {next_label} not found"))?;

    if let Some(active_id) = state.active_tab_id.lock().map_err(lock_err)?.clone() {
        if active_id != id {
            if let Some(active) = app.get_webview(&site_label(&active_id)) {
                let _ = active.hide();
            }
        }
    }

    next.show().map_err(|e| e.to_string())?;
    next.set_focus().map_err(|e| e.to_string())?;
    *state.active_tab_id.lock().map_err(lock_err)? = Some(id);
    Ok(())
}

#[tauri::command]
pub fn close_site_webview(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    let label = site_label(&id);
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|e| e.to_string())?;
    }

    let mut active = state.active_tab_id.lock().map_err(lock_err)?;
    if active.as_deref() == Some(&id) {
        *active = None;
    }
    Ok(())
}

#[tauri::command]
pub fn get_site_webview_ids(app: AppHandle) -> Vec<String> {
    app.webviews()
        .into_keys()
        .filter_map(|label| label.strip_prefix(SITE_PREFIX).map(ToString::to_string))
        .collect()
}

#[tauri::command]
pub fn start_keep_alive(
    app: AppHandle,
    state: tauri::State<AppState>,
    interval_secs: u64,
) -> Result<(), String> {
    stop_keep_alive(app.clone(), state.clone())?;
    let config = Arc::new(load_config_sync()?.tabs);
    let interval_secs = interval_secs.max(30);
    let app_for_task = app.clone();

    let handle = tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        run_keep_alive_cycle(&app_for_task, &config).await;
        loop {
            interval.tick().await;
            run_keep_alive_cycle(&app_for_task, &config).await;
        }
    });

    *state.keep_alive_handle.lock().map_err(lock_err)? = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn stop_keep_alive(_app: AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    if let Some(handle) = state.keep_alive_handle.lock().map_err(lock_err)?.take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
pub fn ping_webview(
    app: AppHandle,
    id: String,
    expected_hostname: String,
) -> Result<String, String> {
    let webview = app
        .get_webview(&site_label(&id))
        .ok_or_else(|| format!("webview site-{id} not found"))?;
    let status = ping_one_webview(&webview, &expected_hostname);
    Ok(status.to_string())
}

pub fn load_config_sync() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = default_config();
        save_config_sync(&config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid config {}: {e}", path.display()))
}

pub fn create_site_webview_on_window<R: Runtime>(
    window: &Window<R>,
    id: &str,
    url: &str,
) -> Result<(), String> {
    let label = site_label(id);
    if window.app_handle().get_webview(&label).is_some() {
        return Ok(());
    }

    let parsed: Url = url.parse().map_err(|e| format!("invalid URL for {id}: {e}"))?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let logical = size.to_logical::<f64>(scale);
    let width = logical.width.max(1.0);
    let height = (logical.height - TAB_BAR_HEIGHT).max(1.0);

    let builder = tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(parsed));
    window
        .add_child(
            builder,
            LogicalPosition::new(0.0, TAB_BAR_HEIGHT),
            LogicalSize::new(width, height),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_all_bounds(app: &AppHandle, window: &Window, tab_h: f64) {
    let Ok(size) = window.inner_size() else { return };
    let Ok(scale) = window.scale_factor() else { return };
    let logical = size.to_logical::<f64>(scale);
    let w = logical.width.max(1.0);
    let h = logical.height.max(tab_h + 1.0);

    if let Some(tabbar) = app.get_webview("tabbar") {
        let _ = tabbar.set_bounds(tauri::Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(w, tab_h).into(),
        });
    }

    for (label, webview) in app.webviews() {
        if label.starts_with(SITE_PREFIX) {
            let _ = webview.set_bounds(tauri::Rect {
                position: LogicalPosition::new(0.0, tab_h).into(),
                size: LogicalSize::new(w, (h - tab_h).max(1.0)).into(),
            });
        }
    }
}

async fn run_keep_alive_cycle(app: &AppHandle, tabs: &[TabConfig]) {
    for tab in tabs {
        let Some(webview) = app.get_webview(&site_label(&tab.id)) else {
            continue;
        };
        let status = ping_one_webview(&webview, &tab.expected_hostname);
        let _ = app.emit_to(
            "tabbar",
            "session-status",
            serde_json::json!({ "id": tab.id, "status": status }),
        );
    }

    let _ = app.emit_to("tabbar", "keep-alive-cycle-done", serde_json::json!({}));
}

fn ping_one_webview<R: Runtime>(webview: &tauri::Webview<R>, expected_hostname: &str) -> &'static str {
    match webview.url() {
        Ok(url) => {
            let host = url.host_str().unwrap_or("");
            if host == expected_hostname || host.ends_with(&format!(".{expected_hostname}")) {
                let current = url.as_str().replace('\\', "\\\\").replace('\'', "\\'");
                let js = format!(
                    "fetch('{current}', {{ method: 'HEAD', credentials: 'include', cache: 'no-store' }}).catch(() => {{}});"
                );
                let _ = webview.eval(js);
                "active"
            } else {
                "expired"
            }
        }
        Err(_) => "error",
    }
}

fn save_config_sync(config: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

fn config_path() -> Result<PathBuf, String> {
    let mut base = dirs::config_dir().ok_or_else(|| "config directory not found".to_string())?;
    base.push("webcode-session-keeper");
    base.push("config.json");
    Ok(base)
}

fn site_label(id: &str) -> String {
    format!("{SITE_PREFIX}{id}")
}

fn lock_err<T>(err: std::sync::PoisonError<T>) -> String {
    err.to_string()
}

fn default_config() -> AppConfig {
    AppConfig {
        keep_alive_interval_secs: 600,
        tabs: vec![
            TabConfig {
                id: "google-calendar".to_string(),
                name: "Google Calendar".to_string(),
                url: "https://calendar.google.com/calendar/u/0/r".to_string(),
                expected_hostname: "calendar.google.com".to_string(),
            },
            TabConfig {
                id: "tencent-docs".to_string(),
                name: "腾讯文档".to_string(),
                url: "https://doc.weixin.qq.com/sheet/e3_AAkAnQYYAAobkO1wc4NRumAOQCo6a".to_string(),
                expected_hostname: "doc.weixin.qq.com".to_string(),
            },
            TabConfig {
                id: "cloudflare".to_string(),
                name: "Cloudflare".to_string(),
                url: "https://dash.cloudflare.com/0aa088497f85e67a7eae3fbe77521797/".to_string(),
                expected_hostname: "dash.cloudflare.com".to_string(),
            },
            TabConfig {
                id: "webstore-ratings".to_string(),
                name: "Web Store 控制台".to_string(),
                url: "https://chrome.google.com/webstore/devconsole".to_string(),
                expected_hostname: "chrome.google.com".to_string(),
            },
            TabConfig {
                id: "extension-reviews".to_string(),
                name: "扩展评价".to_string(),
                url: "https://chromewebstore.google.com/detail/reviews?hl=zh-CN".to_string(),
                expected_hostname: "chromewebstore.google.com".to_string(),
            },
        ],
    }
}

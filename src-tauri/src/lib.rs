mod commands;

use commands::tab_commands::{
    create_site_webview_on_window, load_config_sync, update_all_bounds, AppState, MAIN_WINDOW_LABEL,
    TAB_BAR_HEIGHT,
};
use commands::tab_commands::*;
use tauri::{LogicalPosition, LogicalSize, Manager, TitleBarStyle, WebviewUrl};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            create_site_webview,
            show_site_webview,
            close_site_webview,
            get_site_webview_ids,
            start_keep_alive,
            stop_keep_alive,
            ping_webview,
        ])
        .setup(|app| {
            let config = load_config_sync().expect("failed to load session keeper config");

            let main_window = tauri::window::WindowBuilder::new(app, MAIN_WINDOW_LABEL)
                .title("Session Keeper")
                .inner_size(1280.0, 800.0)
                .min_inner_size(760.0, 420.0)
                .resizable(true)
                .transparent(true)
                .title_bar_style(TitleBarStyle::Transparent)
                .build()?;

            for tab in &config.tabs {
                create_site_webview_on_window(&main_window, &tab.id, &tab.url)
                    .expect("failed to create site webview");
                if let Some(webview) = app.get_webview(&format!("site-{}", tab.id)) {
                    let _ = webview.hide();
                }
            }

            let tabbar = tauri::webview::WebviewBuilder::new(
                "tabbar",
                WebviewUrl::App("index.html".into()),
            )
            .transparent(true);
            main_window.add_child(
                tabbar,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1280.0, TAB_BAR_HEIGHT),
            )?;

            let app_handle = app.handle().clone();
            let window_for_resize = main_window.clone();
            main_window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Resized(_)) {
                    update_all_bounds(&app_handle, &window_for_resize, TAB_BAR_HEIGHT);
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {});
}

mod clipboard;
mod command_schema;
mod commands;
mod extensions;
mod frecency;
mod icons;
mod launcher;
mod providers;
mod settings;
mod shortcuts;
mod undo;
mod window;

use std::sync::Arc;
use tokio::sync::RwLock;

use clipboard::ClipboardState;
use extensions::color_picker::ColorPickerProvider;
use extensions::notion::NotionProvider;
use extensions::password_generator::{PasswordGeneratorProvider, SharedVaultSession, VaultSession};
use extensions::perf_monitor::{PerfMonitorProvider, SharedAlertConfig};
use extensions::perf_store::SharedMetricsStore;
use extensions::window_management::{SharedWmState, WmState, WindowManagementProvider};
use extensions::screenshot::ScreenshotProvider;
use extensions::ruler::RulerProvider;
use extensions::registry::ExtensionRegistry;
use frecency::FrecencyTracker;
use launcher::CommandRegistry;
use shortcuts::{SharedShortcutRegistry, ShortcutRegistry};
use undo::UndoStack;
use providers::{
    applications::ApplicationProvider,
    builtin::BuiltinProvider,
    clipboard::ClipboardProvider,
    files::FileSearchProvider,
    CommandProvider,
};
use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    // Check if this is a registered extension shortcut
                    if let Some(registry) = app.try_state::<SharedShortcutRegistry>() {
                        let registry = registry.inner().clone();
                        let app_clone = app.clone();
                        let shortcut_clone = shortcut.clone();
                        tauri::async_runtime::spawn(async move {
                            let reg = registry.read().await;
                            if let Some(id) = reg.resolve_shortcut(&shortcut_clone) {
                                match id.as_str() {
                                    "ruler.open" => {
                                        let _ = extensions::ruler::ruler_open(app_clone);
                                    }
                                    "perf.dashboard" => {
                                        // Toggle main window and emit event to switch to perf view
                                        window::toggle(&app_clone);
                                        use tauri::Emitter;
                                        let _ = app_clone.emit("navigate-view", "perf");
                                    }
                                    _ => {}
                                }
                                return;
                            }
                            // If not a registered shortcut, treat as main toggle
                            window::toggle(&app_clone);
                        });
                    } else {
                        window::toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Extension registry
            let ext_registry = Arc::new(RwLock::new(ExtensionRegistry::load()));
            app.manage(Arc::clone(&ext_registry));

            // Performance monitor store (created early so provider can access it)
            let metrics_store: SharedMetricsStore =
                Arc::new(RwLock::new(extensions::perf_store::MetricsStore::load_from_disk()));

            // Build command registry with all providers
            let mut registry = CommandRegistry::new();
            let providers: Vec<Box<dyn CommandProvider>> = vec![
                Box::new(BuiltinProvider::new()),
                Box::new(ApplicationProvider::new()),
                Box::new(FileSearchProvider::new()),
                Box::new(ClipboardProvider::new()),
                Box::new(NotionProvider::new(Arc::clone(&ext_registry))),
                Box::new(ColorPickerProvider::new()),
                Box::new(PasswordGeneratorProvider::new()),
                Box::new(WindowManagementProvider::new()),
                Box::new(ScreenshotProvider::new()),
                Box::new(RulerProvider::new()),
                Box::new(PerfMonitorProvider::with_store(Arc::clone(&metrics_store))),
            ];

            // Collect shortcuts from all providers before registering them
            let saved = settings::Settings::load();
            let mut shortcut_registry = ShortcutRegistry::new();
            for provider in &providers {
                for binding in provider.shortcuts() {
                    shortcut_registry.register(binding, &saved.shortcuts);
                }
            }

            // Register all providers
            for provider in providers {
                registry.register(provider);
            }

            let registry = Arc::new(RwLock::new(registry));

            // Password generator vault session
            let vault_session: SharedVaultSession =
                Arc::new(RwLock::new(VaultSession::default()));
            app.manage(vault_session);

            // Window management state (tracks last-focused window)
            let wm_state: SharedWmState = Arc::new(RwLock::new(WmState::default()));
            app.manage(wm_state);
            let registry_clone = Arc::clone(&registry);

            // Refresh cache + extract icons in the background
            tauri::async_runtime::spawn(async move {
                let mut reg = registry_clone.write().await;
                reg.refresh_cache().await;
                log::info!(
                    "Command registry loaded: {} static commands",
                    reg.search("").await.len()
                );
                reg.enrich_icons().await;
                log::info!("Icon extraction complete");
            });

            app.manage(registry);

            // Frecency tracker
            let frecency = Arc::new(RwLock::new(FrecencyTracker::load()));
            app.manage(frecency);

            // Undo stack
            let undo_stack = Arc::new(RwLock::new(UndoStack::new()));
            app.manage(undo_stack);

            // Clipboard monitoring
            let clip_state = Arc::new(RwLock::new(ClipboardState::load()));
            clipboard::start_monitor(Arc::clone(&clip_state));
            app.manage(clip_state);

            // Performance monitor collector + alerts
            let alert_config: SharedAlertConfig =
                Arc::new(RwLock::new(extensions::perf_monitor::load_alert_config()));
            extensions::perf_monitor::start_collector(
                app.handle().clone(),
                Arc::clone(&metrics_store),
                Arc::clone(&alert_config),
            );
            app.manage(metrics_store);
            app.manage(alert_config);

            // Shortcut registry (managed state)
            let shortcut_registry: SharedShortcutRegistry =
                Arc::new(RwLock::new(shortcut_registry));
            app.manage(Arc::clone(&shortcut_registry));

            #[cfg(target_os = "macos")]
            {
                if !saved.show_in_dock {
                    use objc2::MainThreadMarker;
                    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                    if let Some(mtm) = MainThreadMarker::new() {
                        let ns_app = NSApplication::sharedApplication(mtm);
                        ns_app.setActivationPolicy(
                            NSApplicationActivationPolicy::Accessory,
                        );
                    }
                }
            }

            // Register global shortcuts
            {
                // Main toggle shortcut
                let shortcut = if saved.replace_spotlight {
                    tauri_plugin_global_shortcut::Shortcut::new(Some(Modifiers::META), Code::Space)
                } else {
                    tauri_plugin_global_shortcut::Shortcut::new(Some(Modifiers::ALT), Code::Space)
                };
                app.global_shortcut().register(shortcut)?;

                // Extension shortcuts
                let reg = shortcut_registry.blocking_read();
                for shortcut in reg.tauri_shortcuts() {
                    if let Err(e) = app.global_shortcut().register(shortcut) {
                        log::warn!("Failed to register shortcut: {e}");
                    }
                }
            }

            // System tray
            let tooltip = if saved.replace_spotlight {
                "Emit \u{2014} Cmd+Space to toggle"
            } else {
                "Emit \u{2014} Option+Space to toggle"
            };
            let _tray = TrayIconBuilder::new()
                .tooltip(tooltip)
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")).unwrap())
                .icon_as_template(true)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        window::toggle(tray.app_handle());
                    }
                })
                .build(app)?;

            // Hide window when it loses focus (click outside)
            let handle = app.handle().clone();
            if let Some(w) = app.get_webview_window("main") {
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        window::hide(&handle);
                    }
                });
            }

            window::show_main(app.handle());

            // Floating panel level keeps backdrop-filter active when unfocused
            #[cfg(target_os = "macos")]
            window::set_floating_panel(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::search_static,
            commands::execute_command,
            commands::hide_window,
            commands::get_settings,
            commands::save_settings,
            commands::get_clipboard_history,
            commands::clipboard_copy,
            commands::clipboard_delete,
            commands::clipboard_clear,
            commands::clipboard_get_image,
            commands::get_extensions,
            commands::set_extension_enabled,
            commands::get_extension_settings,
            commands::save_extension_settings,
            commands::notion_get_databases,
            commands::notion_query_database,
            // Slash command system
            commands::search_commands,
            commands::execute_action,
            commands::resolve_param_options,
            commands::undo_last_action,
            // Notion CRUD
            commands::notion_get_database_schema,
            commands::notion_create_page,
            commands::notion_update_page,
            commands::notion_archive_page,
            commands::notion_add_comment,
            commands::notion_search_pages,
            // Color picker
            extensions::color_picker::color_picker_sample_screen,
            extensions::color_picker::color_picker_save_palettes,
            extensions::color_picker::color_picker_load_palettes,
            // Password generator
            extensions::password_generator::pwgen_has_vault,
            extensions::password_generator::pwgen_setup,
            extensions::password_generator::pwgen_unlock,
            extensions::password_generator::pwgen_lock,
            extensions::password_generator::pwgen_is_unlocked,
            extensions::password_generator::pwgen_generate,
            extensions::password_generator::pwgen_save_to_history,
            extensions::password_generator::pwgen_get_history,
            extensions::password_generator::pwgen_delete_history_entry,
            extensions::password_generator::pwgen_clear_history,
            extensions::password_generator::pwgen_copy_password,
            extensions::password_generator::pwgen_set_lock_timeout,
            extensions::password_generator::pwgen_get_lock_timeout,
            // Window management
            extensions::window_management::wm_check_accessibility,
            extensions::window_management::wm_request_accessibility,
            extensions::window_management::wm_list_windows,
            extensions::window_management::wm_snap_window,
            extensions::window_management::wm_snap_focused,
            extensions::window_management::wm_get_app_icon,
            extensions::window_management::wm_get_screen_info,
            // Screenshot
            extensions::screenshot::screenshot_capture_region,
            extensions::screenshot::screenshot_capture_window,
            extensions::screenshot::screenshot_capture_screen,
            extensions::screenshot::screenshot_list,
            extensions::screenshot::screenshot_delete,
            extensions::screenshot::screenshot_copy,
            extensions::screenshot::screenshot_get_image,
            // Pixel ruler
            extensions::ruler::ruler_open,
            extensions::ruler::ruler_close,
            extensions::ruler::ruler_get_all_screens,
            extensions::ruler::ruler_capture_region,
            extensions::ruler::ruler_detect_edges,
            extensions::ruler::ruler_copy_measurements,
            extensions::ruler::ruler_screenshot_overlay,
            // Performance monitor
            extensions::perf_monitor::perf_get_snapshot,
            extensions::perf_monitor::perf_get_history,
            extensions::perf_monitor::perf_get_processes,
            extensions::perf_monitor::perf_get_alerts,
            extensions::perf_monitor::perf_save_alerts,
            extensions::perf_monitor::perf_resize_window,
            // Shortcuts
            shortcuts::get_shortcuts,
            shortcuts::rebind_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

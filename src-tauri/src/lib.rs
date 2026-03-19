mod clipboard;
mod command_schema;
mod commands;
mod extensions;
mod frecency;
mod icons;
mod launcher;
mod providers;
mod settings;
mod undo;
mod window;

use std::sync::Arc;
use tokio::sync::RwLock;

use clipboard::ClipboardState;
use extensions::color_picker::ColorPickerProvider;
use extensions::notion::NotionProvider;
use extensions::password_generator::{PasswordGeneratorProvider, SharedVaultSession, VaultSession};
use extensions::window_management::{SharedWmState, WmState, WindowManagementProvider};
use extensions::registry::ExtensionRegistry;
use frecency::FrecencyTracker;
use launcher::CommandRegistry;
use undo::UndoStack;
use providers::{
    applications::ApplicationProvider,
    builtin::BuiltinProvider,
    clipboard::ClipboardProvider,
    files::FileSearchProvider,
};
use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
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

            // Build command registry with all providers
            let mut registry = CommandRegistry::new();
            registry.register(Box::new(BuiltinProvider::new()));
            registry.register(Box::new(ApplicationProvider::new()));
            registry.register(Box::new(FileSearchProvider::new()));
            registry.register(Box::new(ClipboardProvider::new()));
            registry.register(Box::new(NotionProvider::new(Arc::clone(&ext_registry))));
            registry.register(Box::new(ColorPickerProvider::new()));
            registry.register(Box::new(PasswordGeneratorProvider::new()));
            registry.register(Box::new(WindowManagementProvider::new()));

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

            // Apply dock visibility from saved settings
            #[cfg(target_os = "macos")]
            {
                let saved = settings::Settings::load();
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

            // Register global shortcut: Option+Space
            app.global_shortcut().register(
                tauri_plugin_global_shortcut::Shortcut::new(Some(Modifiers::ALT), Code::Space),
            )?;

            // System tray
            let _tray = TrayIconBuilder::new()
                .tooltip("Emit — Option+Space to toggle")
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

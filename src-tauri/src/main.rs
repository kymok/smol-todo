#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;
mod mutations;
mod watcher;

use std::time::Duration;
use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::create_item,
            commands::update_item,
            commands::set_status,
            commands::move_item,
            commands::delete_item,
            commands::delete_items,
            commands::add_note,
            commands::update_note,
            commands::delete_note,
            commands::merge_item,
            commands::split_item,
            commands::create_collection,
            commands::rename_collection,
            commands::set_collection_color,
            commands::set_collection_archived,
            commands::move_collection,
            commands::clear_items,
            commands::delete_collection,
        ])
        .setup(|app| {
            app.manage(pond_core::TaskStore::open_default());

            let store_dir = pond_core::paths::default_store_path()
                .parent()
                .map(|p| p.to_path_buf());
            if let Some(dir) = store_dir {
                std::fs::create_dir_all(&dir).ok();
                let handle = app.handle().clone();
                // Keep the watcher alive for the app's lifetime.
                match watcher::watch_dir(&dir, Duration::from_millis(150), move || {
                    let _ = handle.emit("store-changed", ());
                }) {
                    Ok(w) => {
                        app.manage(std::sync::Mutex::new(w));
                    }
                    Err(e) => eprintln!("store watcher failed to start: {e}"),
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running pond-tauri");
}

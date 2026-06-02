#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(dead_code)] // removed when Task 5 (commands.rs) references dto
mod dto;

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running pond-tauri");
}

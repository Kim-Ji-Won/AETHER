fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}

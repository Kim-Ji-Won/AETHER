mod native_bridge;

fn main() {
    if std::env::args_os().skip(1).any(|arg| arg == "--native-bridge-check") {
        println!("AETHER_NATIVE_MACOS_MAJOR={}", native_bridge::macos_major_version());
        return;
    }

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}

mod instance;
mod process;
mod settings;
mod state;
mod versions;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            settings::detect_node_cmd,
            instance::list_instances,
            instance::create_instance,
            instance::update_instance,
            instance::delete_instance,
            instance::duplicate_instance,
            instance::open_instance_dir,
            instance::open_data_dir,
            instance::get_instance_log_buffer,
            instance::open_url_in_browser,
            process::start_instance,
            process::stop_instance,
            process::get_running,
            versions::fetch_versions,
            versions::install_version,
            versions::get_installed_version,
            versions::uninstall_version,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 退出时终止所有由启动器拉起的 dsh 进程
            if let tauri::RunEvent::Exit = event {
                process::kill_all(app.state::<AppState>().inner());
            }
        });
}

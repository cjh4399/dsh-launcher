use crate::instance::{self, InstanceMeta};
use crate::settings;
use crate::state::{AppState, ChildHandle, RunningInfo};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::Manager;

/// CREATE_NO_WINDOW：防止 node/taskkill 弹出控制台窗口
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
    pub instance_id: String,
    /// starting | ready | stopped
    pub status: String,
    pub code: Option<i32>,
    pub url: Option<String>,
}

fn emit_status(app: &tauri::AppHandle, s: &InstanceStatus) {
    use tauri::Emitter;
    let _ = app.emit("instance-status", s);
}

fn push_and_emit_log(app: &tauri::AppHandle, id: &str, stream: &str, line: &str) {
    let state = app.state::<AppState>();
    state.push_log(id, line);
    drop(state);
    use tauri::Emitter;
    let _ = app.emit(
        "instance-log",
        serde_json::json!({ "instanceId": id, "stream": stream, "line": line }),
    );
}

/// 从一行日志中提取 dsh 打印的带 token 的 Web UI 地址
fn extract_boot_url(line: &str) -> Option<String> {
    if !line.contains("token=") {
        return None;
    }
    let start = line.find("http://").or_else(|| line.find("https://"))?;
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn port_is_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

fn wait_port_open(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if port_is_open(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/// 写出实例的启动环境文件（由 node --env-file 读取，需 Node ≥ 20.6）。
/// DSH_HOME 指向实例自己的 home，实现 profiles / 凭据 / 配置的完全隔离。
/// 路径统一用正斜杠（Windows API 与 Node 均接受）。
fn write_boot_env(home: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let path = home.join("dsh-launcher.env");
    let home_fwd = home.to_string_lossy().replace('\\', "/");
    let content = "DSH_HOME=".to_string() + &home_fwd + "\n";
    std::fs::create_dir_all(home).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

type BootUrlSlot = Arc<Mutex<Option<String>>>;

fn read_stream<R: std::io::Read + Send + 'static>(
    app: tauri::AppHandle,
    id: String,
    stream_label: &'static str,
    stream: R,
    boot_url_slot: BootUrlSlot,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(url) = extract_boot_url(trimmed) {
                let mut slot = boot_url_slot.lock().unwrap();
                let changed = slot.as_deref() != Some(url.as_str());
                *slot = Some(url.clone());
                drop(slot);
                if changed {
                    let auto_open = settings::global().read().unwrap().auto_open_browser;
                    if auto_open {
                        let _ = tauri_plugin_opener::open_url(&url, None::<&str>);
                    }
                    emit_status(
                        &app,
                        &InstanceStatus {
                            instance_id: id.clone(),
                            status: "ready".into(),
                            code: None,
                            url: Some(url),
                        },
                    );
                }
            }
            push_and_emit_log(&app, &id, stream_label, &trimmed);
        }
    })
}

/// 启动实例：
///   node --env-file=<实例>/home/dsh-launcher.env <实例>/pkg/…/bin.js web --port <端口> --no-open
///
/// 程序名为字面量 `node`（走 PATH，要求 Node ≥ 20.6）；参数只含实例目录内
/// 派生路径、数字端口与固定常量，纯参数列表调用、不经任何 shell。
/// DSH_HOME 经 --env-file 注入，使每个实例拥有独立的 profiles / 凭据 / 配置
/// （模型凭据由 dsh Web UI 写入各实例自己的 home，实例间天然隔离）。
#[tauri::command]
pub fn start_instance(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<RunningInfo, String> {
    {
        let children = state.children.lock().unwrap();
        if children.contains_key(&id) {
            return Err("实例已在运行".into());
        }
    }

    let dir = instance::instance_dir(&id)?;
    if !dir.is_dir() {
        return Err(format!("实例 {id} 不存在"));
    }
    let mut meta: InstanceMeta = instance::load_meta(&dir)?;

    if !crate::versions::bin_entry(&id)?.is_file() {
        return Err("实例尚未安装 dsh，请先到「下载」页为该实例安装版本".into());
    }

    // 端口避让：首选端口被占（其他实例或别的程序）则向后找
    let port = if instance::is_port_free(meta.port) {
        meta.port
    } else {
        instance::find_free_port(meta.port + 1, 100)
    };

    // node 必须可用（检测同时给出 npm-cli.js 位置供安装功能使用）
    settings::detect_node()?;

    let home = dir.join("home");
    let workspace = dir.join("workspace");
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    let env_file = write_boot_env(&home)?;

    let entry = crate::versions::bin_entry(&id)?;
    let port_str = port.to_string();

    let mut cmd = Command::new("node");
    cmd.arg("--env-file")
        .arg(&env_file)
        .arg(&entry)
        .arg("web")
        .arg("--port")
        .arg(&port_str)
        .arg("--no-open")
        .current_dir(&workspace);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("启动失败: {e}"))?;
    let pid = child.id();
    let started_at = AppState::now_millis();

    meta.last_launched_at = Some(started_at);
    let _ = instance::save_meta(&dir, &meta);

    push_and_emit_log(&app, &id, "system", &format!("[launcher] 启动 pid={pid} port={port}"));

    let boot_url_slot: BootUrlSlot = Arc::new(Mutex::new(None));

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // 健康检查：端口通了即就绪（token URL 由日志线程独立上报/打开）
    let app_h = app.clone();
    let id_h = id.clone();
    let slot_h = boot_url_slot.clone();
    std::thread::spawn(move || {
        if wait_port_open(port, Duration::from_secs(180)) {
            let url = slot_h.lock().unwrap().clone();
            emit_status(
                &app_h,
                &InstanceStatus {
                    instance_id: id_h,
                    status: "ready".into(),
                    code: None,
                    url,
                },
            );
        }
    });

    if let Some(s) = stdout {
        read_stream(app.clone(), id.clone(), "stdout", s, boot_url_slot.clone());
    }
    if let Some(s) = stderr {
        read_stream(app.clone(), id.clone(), "stderr", s, boot_url_slot.clone());
    }

    // waiter：进程退出后清理运行表并广播
    let app_w = app.clone();
    let id_w = id.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        let app = app_w;
        let state = app.state::<AppState>();
        {
            let mut children = state.children.lock().unwrap();
            if children.get(&id_w).map(|c| c.pid) == Some(pid) {
                children.remove(&id_w);
            }
        }
        drop(state);
        let code = status.ok().and_then(|c| c.code());
        push_and_emit_log(
            &app,
            &id_w,
            "system",
            &format!("[launcher] 进程退出 code={code:?}"),
        );
        emit_status(
            &app,
            &InstanceStatus {
                instance_id: id_w,
                status: "stopped".into(),
                code,
                url: None,
            },
        );
    });

    let info = RunningInfo {
        instance_id: id.clone(),
        pid,
        port,
        started_at,
        boot_url: None,
    };
    state.children.lock().unwrap().insert(
        id,
        ChildHandle {
            pid,
            port,
            started_at,
            boot_url: Mutex::new(None),
        },
    );
    emit_status(
        &app,
        &InstanceStatus {
            instance_id: info.instance_id.clone(),
            status: "starting".into(),
            code: None,
            url: None,
        },
    );
    Ok(info)
}

/// 停止实例：taskkill 树杀（node 及其子进程）
#[tauri::command]
pub fn stop_instance(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    instance::check_id(&id)?;
    let pid = {
        let children = state.children.lock().unwrap();
        children.get(&id).map(|c| c.pid)
    };
    let pid = pid.ok_or("实例未在运行")?;
    kill_pid(pid)
}

fn kill_pid(pid: u32) -> Result<(), String> {
    let pid_str = pid.to_string();
    let mut cmd = Command::new("taskkill");
    cmd.arg("/F").arg("/T").arg("/PID").arg(&pid_str);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        // 进程已不存在视为成功，交给 waiter 线程清理
        if !msg.contains("没有找到") && !msg.to_lowercase().contains("not found") {
            return Err(format!("taskkill 失败: {}", msg.trim()));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_running(state: tauri::State<'_, AppState>) -> Vec<RunningInfo> {
    let children = state.children.lock().unwrap();
    children
        .iter()
        .map(|(id, c)| RunningInfo {
            instance_id: id.clone(),
            pid: c.pid,
            port: c.port,
            started_at: c.started_at,
            boot_url: c.boot_url.lock().unwrap().clone(),
        })
        .collect()
}

/// 应用退出时终止所有由启动器拉起的实例进程
pub fn kill_all(state: &AppState) {
    let pids: Vec<u32> = {
        let children = state.children.lock().unwrap();
        children.values().map(|c| c.pid).collect()
    };
    for pid in pids {
        let _ = kill_pid(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_boot_url, write_boot_env};

    #[test]
    fn parses_token_url() {
        assert_eq!(
            extract_boot_url("dsh web: http://127.0.0.1:31987/?token=abc def"),
            Some("http://127.0.0.1:31987/?token=abc".into())
        );
        assert_eq!(extract_boot_url("plain log line"), None);
    }

    #[test]
    fn boot_env_content() {
        let tmp = std::env::temp_dir().join("dsh-launcher-test-env");
        let p = write_boot_env(&tmp).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("DSH_HOME="));
        assert!(!text.contains('\\'));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

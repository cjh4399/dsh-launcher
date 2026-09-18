use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const DEFAULT_REGISTRY: &str = "https://registry.npmmirror.com";

/// 全局设置。settings.json 存放在可执行文件旁（便携式，PCL2 风格），
/// data_root 可以被用户挪到任意磁盘位置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// 实例数据根目录；None 时使用 exe 同级 dsh-data
    #[serde(alias = "data_root")]
    pub data_root: Option<String>,
    /// npm registry；None 时使用 npmmirror
    pub registry: Option<String>,
    pub theme: String,
    #[serde(alias = "auto_open_browser")]
    pub auto_open_browser: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            data_root: None,
            registry: None,
            theme: "dark".into(),
            auto_open_browser: true,
        }
    }
}

fn settings_path() -> PathBuf {
    exe_dir().join("dsh-launcher.json")
}

/// 进程级全局设置。Tauri 命令与各模块都从这里读写，
/// 写入时同步落盘到 settings.json。
static GLOBAL_SETTINGS: OnceLock<RwLock<Settings>> = OnceLock::new();

pub fn global() -> &'static RwLock<Settings> {
    GLOBAL_SETTINGS.get_or_init(|| RwLock::new(Settings::load()))
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Settings {
    pub fn load() -> Self {
        let p = settings_path();
        if let Ok(text) = fs::read_to_string(&p) {
            match serde_json::from_str::<Settings>(&text) {
                Ok(s) => return s,
                Err(e) => eprintln!("settings.json 解析失败，使用默认设置: {e}"),
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let p = settings_path();
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&p, text).map_err(|e| e.to_string())
    }

    pub fn resolve_data_root(&self) -> PathBuf {
        match &self.data_root {
            Some(d) if !d.trim().is_empty() => PathBuf::from(d),
            _ => exe_dir().join("dsh-data"),
        }
    }

    pub fn registry_url(&self) -> String {
        match &self.registry {
            Some(r) if !r.trim().is_empty() => r.trim().trim_end_matches('/').to_string(),
            _ => DEFAULT_REGISTRY.into(),
        }
    }
}

// ---------- settings commands ----------

#[tauri::command]
pub fn get_settings() -> Settings {
    global().read().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<Settings, String> {
    {
        let mut s = global().write().unwrap();
        *s = settings;
        s.save()?;
    }
    Ok(global().read().unwrap().clone())
}

/// 检测 Node.js 安装位置（不执行任何用户提供的路径）：
/// 1. `where node`（程序与参数均为字面量）
/// 2. 常见安装位置的文件存在性检查
/// 返回 (node.exe 绝对路径, 版本号)。启动器以字面量 `node` 程序名
/// 拉起子进程（走 PATH），此函数的结果用于定位 npm-cli.js 与界面展示。
pub fn detect_node() -> Result<(PathBuf, String), String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(out) = std::process::Command::new("where")
        .arg("node")
        .creation_flags(crate::process::CREATE_NO_WINDOW)
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let p = PathBuf::from(line.trim());
                if p.is_file() {
                    candidates.push(p);
                }
            }
        }
    }
    for extra in [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ] {
        let p = PathBuf::from(extra);
        if p.is_file() {
            candidates.push(p);
        }
    }

    for c in candidates {
        // 只做文件名合理性判断；版本号通过 `node --version`（字面量程序）读取
        if let Some(name) = c.file_name().map(|s| s.to_string_lossy().to_lowercase()) {
            if name != "node.exe" && name != "node" {
                continue;
            }
        } else {
            continue;
        }
        if let Ok(out) = std::process::Command::new("node")
            .arg("--version")
            .creation_flags(crate::process::CREATE_NO_WINDOW)
            .output()
        {
            if out.status.success() {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return Ok((c, if v.is_empty() { "unknown".into() } else { v }));
            }
        }
    }
    Err("未找到 Node.js：请安装 Node.js 并确保其在 PATH 中（启动器通过 PATH 中的 node 运行实例）".into())
}

#[tauri::command]
pub fn detect_node_cmd() -> Result<NodeInfo, String> {
    let (path, version) = detect_node()?;
    Ok(NodeInfo { path: path.to_string_lossy().into_owned(), version })
}

#[derive(serde::Serialize)]
pub struct NodeInfo {
    pub path: String,
    pub version: String,
}

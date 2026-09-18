use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 一个实例 = 一个完全隔离的 dsh 运行环境：
///   <data_root>/instances/<id>/
///     instance.json   元数据
///     pkg/            该实例独立安装的 @deepseek-ai/dsh（npm --prefix）
///     home/           作为 DSH_HOME 传给 dsh（profiles / 凭据 / 配置都在这里）
///     workspace/      进程工作目录（agent 的工作区）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceMeta {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: Option<String>,
    pub port: u16,
    #[serde(alias = "dsh_version")]
    pub dsh_version: Option<String>,
    #[serde(alias = "created_at")]
    pub created_at: i64,
    #[serde(alias = "last_launched_at")]
    pub last_launched_at: Option<i64>,
}

impl Default for InstanceMeta {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            icon: "🚀".into(),
            color: None,
            port: 3080,
            dsh_version: None,
            created_at: 0,
            last_launched_at: None,
        }
    }
}

pub fn instances_root() -> PathBuf {
    let root = crate::settings::global()
        .read()
        .unwrap()
        .resolve_data_root()
        .join("instances");
    let _ = fs::create_dir_all(&root);
    root
}

/// 实例 id 白名单：只允许字母数字-_（生成器 sanitize_slug 保证同一字符集）。
/// 这是防路径穿越的中央校验点：id 来自前端，绝不参与 `..`、分隔符等路径语义。
pub fn check_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 80 {
        return Err("实例 id 非法".into());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("实例 id 含非法字符".into());
    }
    Ok(())
}

pub fn instance_dir(id: &str) -> Result<PathBuf, String> {
    check_id(id)?;
    Ok(instances_root().join(id))
}

pub fn order_file() -> PathBuf {
    crate::settings::global()
        .read()
        .unwrap()
        .resolve_data_root()
        .join("instance-order.json")
}

fn meta_path(dir: &Path) -> PathBuf {
    dir.join("instance.json")
}

pub fn load_meta(dir: &Path) -> Result<InstanceMeta, String> {
    let text = fs::read_to_string(meta_path(dir))
        .map_err(|e| format!("读取 instance.json 失败: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("instance.json 解析失败: {e}"))
}

pub fn save_meta(dir: &Path, meta: &InstanceMeta) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(meta_path(dir), text).map_err(|e| e.to_string())
}

fn load_order() -> Vec<String> {
    fs::read_to_string(order_file())
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
}

fn save_order(order: &[String]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(order).map_err(|e| e.to_string())?;
    fs::write(order_file(), text).map_err(|e| e.to_string())
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 从名称生成合法目录 id：只保留字母数字-_，其余替换为 '-'
pub fn sanitize_slug(name: &str) -> String {
    let mut out: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    while out.starts_with('-') {
        out.remove(0);
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out = "instance".into();
    }
    out
}

fn gen_id(name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0);
    format!("{}-{:05x}", sanitize_slug(name), nanos & 0xfffff)
}

/// 端口是否空闲
pub fn is_port_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn find_free_port(start: u16, span: u16) -> u16 {
    for p in start..start.saturating_add(span) {
        if is_port_free(p) {
            return p;
        }
    }
    start
}

// ---------- commands ----------

#[tauri::command]
pub fn list_instances() -> Result<Vec<InstanceMeta>, String> {
    let root = instances_root();
    let order = load_order();
    let mut metas: Vec<InstanceMeta> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for id in &order {
        let dir = root.join(id);
        if dir.join("instance.json").is_file() {
            if let Ok(m) = load_meta(&dir) {
                seen.insert(m.id.clone());
                metas.push(m);
            }
        }
    }
    // 兜底：目录存在但不在 order 里（手动复制进来的实例）
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            let dir = e.path();
            if dir.join("instance.json").is_file() {
                if let Ok(m) = load_meta(&dir) {
                    if seen.insert(m.id.clone()) {
                        metas.push(m);
                    }
                }
            }
        }
    }
    Ok(metas)
}

#[tauri::command]
pub fn create_instance(
    name: String,
    icon: Option<String>,
    port: Option<u16>,
) -> Result<InstanceMeta, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("实例名称不能为空".into());
    }
    if name.chars().count() > 40 {
        return Err("实例名称过长".into());
    }

    let root = instances_root();
    let id = {
        let mut candidate = gen_id(&name);
        while root.join(&candidate).exists() {
            candidate = gen_id(&name);
        }
        candidate
    };

    let port = match port {
        Some(p) if p >= 1024 => {
            // 与现有实例的端口错开
            let others = list_instances()?;
            if others.iter().any(|m| m.port == p) || !is_port_free(p) {
                find_free_port(p + 1, 200)
            } else {
                p
            }
        }
        _ => {
            let used: Vec<u16> = list_instances()?.iter().map(|m| m.port).collect();
            let mut p = find_free_port(3080, 200);
            while used.contains(&p) {
                p = find_free_port(p + 1, 200);
            }
            p
        }
    };

    let dir = root.join(&id);
    for sub in ["pkg", "home", "workspace"] {
        fs::create_dir_all(dir.join(sub)).map_err(|e| e.to_string())?;
    }
    // pkg 里预置 package.json，npm --prefix 安装时行为更稳定
    fs::write(
        dir.join("pkg").join("package.json"),
        "{\n  \"name\": \"dsh-instance-pkg\",\n  \"private\": true\n}\n",
    )
    .map_err(|e| e.to_string())?;

    let meta = InstanceMeta {
        id: id.clone(),
        name,
        icon: icon.unwrap_or_else(|| "🚀".into()),
        color: None,
        port,
        dsh_version: None,
        created_at: now_millis(),
        last_launched_at: None,
    };
    save_meta(&dir, &meta)?;

    let mut order = load_order();
    order.insert(0, id);
    save_order(&order)?;
    Ok(meta)
}

#[tauri::command]
pub fn update_instance(
    id: String,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    port: Option<u16>,
) -> Result<InstanceMeta, String> {
    let dir = instance_dir(&id)?;
    if !dir.exists() {
        return Err(format!("实例 {id} 不存在"));
    }
    let mut meta = load_meta(&dir)?;
    if let Some(n) = name {
        let n = n.trim().to_string();
        if n.is_empty() {
            return Err("实例名称不能为空".into());
        }
        meta.name = n;
    }
    if let Some(i) = icon {
        meta.icon = i;
    }
    if let Some(c) = color {
        meta.color = if c.is_empty() { None } else { Some(c) };
    }
    if let Some(p) = port {
        if p < 1024 {
            return Err("端口需在 1024-65535 之间".into());
        }
        meta.port = p;
    }
    save_meta(&dir, &meta)?;
    Ok(meta)
}

#[tauri::command]
pub fn delete_instance(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    if state.children.lock().unwrap().contains_key(&id) {
        return Err("实例正在运行，请先停止再删除".into());
    }
    let dir = instance_dir(&id)?;
    if !dir.exists() {
        return Err(format!("实例 {id} 不存在"));
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("删除目录失败: {e}"))?;
    let order: Vec<String> = load_order().into_iter().filter(|x| x != &id).collect();
    save_order(&order)?;
    state.logs.lock().unwrap().remove(&id);
    Ok(())
}

/// 复制实例：拷贝配置与工作区（home/workspace），跳过 pkg 里的依赖
/// （node_modules 体积大且可随时从下载页重装）
#[tauri::command]
pub fn duplicate_instance(id: String, new_name: String) -> Result<InstanceMeta, String> {
    let src = instance_dir(&id)?;
    if !src.exists() {
        return Err(format!("实例 {id} 不存在"));
    }
    let meta = create_instance(new_name, None, None)?;
    let dst = instance_dir(&meta.id)?;
    for sub in ["home", "workspace"] {
        copy_dir_recursive(&src.join(sub), &dst.join(sub))?;
    }
    let mut new_meta = meta.clone();
    if let Ok(old) = load_meta(&src) {
        new_meta.icon = old.icon.clone();
        new_meta.color = old.color.clone();
    }
    save_meta(&dst, &new_meta)?;
    Ok(new_meta)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn open_instance_dir(id: String) -> Result<(), String> {
    let dir = instance_dir(&id)?;
    if !dir.exists() {
        return Err(format!("实例 {id} 不存在"));
    }
    tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_data_dir() -> Result<(), String> {
    let root = crate::settings::global().read().unwrap().resolve_data_root();
    let _ = fs::create_dir_all(&root);
    tauri_plugin_opener::open_path(&root, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_instance_log_buffer(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Vec<String> {
    state.log_buffer(&id)
}

/// dsh 启动日志会打印带 token 的真实 URL，前端拿这个直接开浏览器最稳
#[tauri::command]
pub fn open_url_in_browser(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings;

    fn use_temp_root(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir()
            .join(format!("dsh-launcher-it-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        settings::global().write().unwrap().data_root =
            Some(tmp.to_string_lossy().into_owned());
        tmp
    }

    #[test]
    fn instance_lifecycle() {
        let tmp = use_temp_root("lifecycle");

        // 创建（名称含非法字符，id 应被 slug 化；目录布局完整）
        let meta = create_instance("测试 实例 One".into(), Some("🤖".into()), None).unwrap();
        let dir = instance_dir(&meta.id).unwrap();
        assert!(dir.join("pkg").is_dir());
        assert!(dir.join("home").is_dir());
        assert!(dir.join("workspace").is_dir());
        assert!(dir.join("pkg").join("package.json").is_file());
        assert_eq!(meta.icon, "🤖");
        assert!((1024..=65535).contains(&meta.port));

        // 元数据可回读
        let round = load_meta(&dir).unwrap();
        assert_eq!(round.id, meta.id);
        assert_eq!(round.port, meta.port);

        // 更新
        let updated =
            update_instance(meta.id.clone(), Some("改名".into()), None, None, Some(13000))
                .unwrap();
        assert_eq!(updated.name, "改名");
        assert_eq!(updated.port, 13000);

        // 端口被占用时自动避让
        let second = create_instance("second".into(), None, Some(13000)).unwrap();
        assert_ne!(second.port, 13000);

        // 复制（拷贝 home/workspace）
        let dup = duplicate_instance(meta.id.clone(), "dup".into()).unwrap();
        assert!(instance_dir(&dup.id).unwrap().join("workspace").is_dir());

        // 删除（delete 命令的文件操作部分）
        assert!(dir.exists());
        fs::remove_dir_all(&dir).unwrap();
        let order: Vec<String> = load_order().into_iter().filter(|x| x != &meta.id).collect();
        save_order(&order).unwrap();
        assert!(!dir.exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn id_validation_blocks_traversal() {
        assert!(check_id("../evil").is_err());
        assert!(check_id("a/b").is_err());
        assert!(check_id("a\\b").is_err());
        assert!(check_id("a b").is_err());
        assert!(check_id("").is_err());
        assert!(check_id("abc-123_DEF").is_ok());

        // instance_dir 拒绝穿越：绝不拼出数据目录之外的路径
        assert!(instance_dir("../outside").is_err());
        assert!(instance_dir("C:\\Windows").is_err());
    }

    #[test]
    fn meta_json_contract() {
        // 前端契约：字段必须是 camelCase
        let m = InstanceMeta {
            id: "x".into(),
            dsh_version: Some("1.2.3".into()),
            created_at: 42,
            ..Default::default()
        };
        let v = serde_json::to_value(&m).unwrap();
        assert!(v.get("dshVersion").is_some(), "{v}");
        assert!(v.get("createdAt").is_some(), "{v}");
        assert!(v.get("dsh_version").is_none(), "{v}");

        // 兼容旧版 snake_case 元数据文件，升级不丢版本记录
        let old: InstanceMeta = serde_json::from_str(
            r#"{"id":"old","dsh_version":"0.1.6-alpha.2","created_at":1}"#,
        )
        .unwrap();
        assert_eq!(old.dsh_version.as_deref(), Some("0.1.6-alpha.2"));
    }
}

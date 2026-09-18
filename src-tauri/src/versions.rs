use crate::instance;
use crate::process::CREATE_NO_WINDOW;
use crate::settings::global;
use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const DSH_PACKAGE: &str = "@deepseek-ai/dsh";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: String,
    /// latest=推荐(最新 tag) / rc / alpha / stable
    pub channel: String,
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub instance_id: String,
    pub version: Option<String>,
    pub phase: String, // resolving | installing | done | error
    pub line: Option<String>,
}

fn pkg_dir(id: &str) -> Result<PathBuf, String> {
    Ok(instance::instance_dir(id)?.join("pkg"))
}

pub fn installed_pkg_json(id: &str) -> Result<PathBuf, String> {
    Ok(pkg_dir(id)?
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("package.json"))
}

fn read_installed_version(id: &str) -> Result<String, String> {
    let p = installed_pkg_json(id)?;
    if !p.is_file() {
        return Err("实例尚未安装 dsh".into());
    }
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(v["version"].as_str().unwrap_or("unknown").to_string())
}

/// dsh 的 bin 入口（实测 0.1.x 均为 lib/bin.js）
pub fn bin_entry(id: &str) -> Result<PathBuf, String> {
    Ok(pkg_dir(id)?
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js"))
}

/// registry 设置白名单：仅允许 http(s) URL，杜绝任意字符串进入子进程参数
fn valid_registry(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 200
        && url.chars().all(|c| !c.is_whitespace() && !c.is_control())
}

/// 把目标版本写进 pkg/package.json 的 dependencies，
/// 版本号通过 JSON 文件而非命令行传递（精确锁定该版本，含预发布号）。
fn write_pkg_manifest(id: &str, version: &str) -> Result<PathBuf, String> {
    let pkg = pkg_dir(id)?;
    fs::create_dir_all(&pkg).map_err(|e| e.to_string())?;
    let mut deps = serde_json::Map::new();
    deps.insert(
        DSH_PACKAGE.to_string(),
        serde_json::Value::String(version.to_string()),
    );
    let manifest = serde_json::json!({
        "name": "dsh-instance-pkg",
        "private": true,
        "dependencies": serde_json::Value::Object(deps)
    });
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    let path = pkg.join("package.json");
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(pkg)
}

fn classify_channel(version: &str, latest_tag: Option<&str>) -> String {
    if Some(version) == latest_tag {
        "latest".into()
    } else if version.contains("alpha") {
        "alpha".into()
    } else if version.contains('-') {
        // 0.1.x 时期基本全是 rc / pre-release
        "rc".into()
    } else {
        "stable".into()
    }
}

fn iso_to_millis(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}

#[tauri::command]
pub async fn fetch_versions() -> Result<Vec<VersionInfo>, String> {
    let registry = global().read().unwrap().registry_url();
    if !valid_registry(&registry) {
        return Err("registry 设置非法（仅允许 http/https 地址）".into());
    }
    let url = format!("{}/@deepseek-ai%2Fdsh", registry.trim_end_matches('/'));

    let resp = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("registry 返回 {}", resp.status()));
    }
    let doc: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let latest_tag = doc["dist-tags"]["latest"].as_str().map(|s| s.to_string());
    let versions = doc["versions"]
        .as_object()
        .ok_or("registry 数据缺少 versions 字段")?;
    let time = doc["time"].as_object();

    let mut out: Vec<VersionInfo> = Vec::new();
    for (v, body) in versions {
        let version = body["version"].as_str().unwrap_or(v).to_string();
        let published_at = time
            .and_then(|t| t.get(v))
            .and_then(|t| t.as_str())
            .and_then(iso_to_millis);
        out.push(VersionInfo {
            channel: classify_channel(&version, latest_tag.as_deref()),
            version,
            published_at,
        });
    }

    out.sort_by(|a, b| {
        let (va, vb) = (
            semver::Version::parse(&a.version),
            semver::Version::parse(&b.version),
        );
        match (va, vb) {
            (Ok(x), Ok(y)) => y.cmp(&x),
            _ => b.version.cmp(&a.version),
        }
    });
    Ok(out)
}

fn emit_progress(app: &tauri::AppHandle, p: &InstallProgress) {
    use tauri::Emitter;
    let _ = app.emit("install-progress", p);
}

fn spawn_pipe_reader<R: std::io::Read + Send + 'static>(
    app: tauri::AppHandle,
    instance_id: String,
    version: String,
    stream: R,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            emit_progress(
                &app,
                &InstallProgress {
                    instance_id: instance_id.clone(),
                    version: Some(version.clone()),
                    phase: "installing".into(),
                    line: Some(trimmed.to_string()),
                },
            );
        }
    })
}

/// 在实例 pkg/ 内安装指定版本。
/// 版本号写入 package.json 后执行参数固定的 `npm install`，
/// 命令行不含任何用户输入；输出逐行以 install-progress 事件推送。
#[tauri::command]
pub async fn install_version(
    app: tauri::AppHandle,
    instance_id: String,
    version: String,
) -> Result<String, String> {
    let dir = instance::instance_dir(&instance_id)?;
    if !dir.is_dir() {
        return Err(format!("实例 {instance_id} 不存在"));
    }
    // 必须是合法 semver（杜绝特殊字符进入 package.json / 文件路径）
    let parsed = semver::Version::parse(&version).map_err(|_| format!("非法版本号: {version}"))?;
    let version = parsed.to_string();

    let registry = global().read().unwrap().registry_url();
    if !valid_registry(&registry) {
        return Err("registry 设置非法（仅允许 http/https 地址）".into());
    }
    let pkg = write_pkg_manifest(&instance_id, &version)?;

    emit_progress(
        &app,
        &InstallProgress {
            instance_id: instance_id.clone(),
            version: Some(version.clone()),
            phase: "resolving".into(),
            line: Some(format!("准备安装 {DSH_PACKAGE}@{version} …")),
        },
    );

    // 定位 npm-cli.js（node 安装目录自带），程序本体用字面量 `node`
    let node_dir = crate::settings::detect_node()?.0;
    let npm_cli = node_dir
        .parent()
        .map(|d| d.join("node_modules/npm/bin/npm-cli.js"))
        .filter(|p| p.is_file())
        .ok_or_else(|| {
            "未找到 npm-cli.js（node 安装目录下缺少 node_modules/npm）".to_string()
        })?;

    let mut cmd = Command::new("node");
    cmd.arg(&npm_cli)
        .arg("install")
        .arg("--prefix")
        .arg(&pkg)
        .arg("--registry")
        .arg(&registry)
        .arg("--no-audit")
        .arg("--no-fund")
        .arg("--loglevel")
        .arg("warn");

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    emit_progress(
        &app,
        &InstallProgress {
            instance_id: instance_id.clone(),
            version: Some(version.clone()),
            phase: "installing".into(),
            line: Some("正在下载依赖（首次可能需要几分钟）…".into()),
        },
    );

    // 阻塞安装放到独立线程，避免卡住 Tauri 的异步运行时
    let app2 = app.clone();
    let instance_id2 = instance_id.clone();
    let version2 = version.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let mut handles = Vec::new();
        if let Some(s) = stdout {
            handles.push(spawn_pipe_reader(
                app2.clone(),
                instance_id2.clone(),
                version2.clone(),
                s,
            ));
        }
        if let Some(s) = stderr {
            handles.push(spawn_pipe_reader(
                app2.clone(),
                instance_id2.clone(),
                version2.clone(),
                s,
            ));
        }
        for h in handles {
            let _ = h.join();
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("npm 安装失败（退出码 {:?}）", status.code()));
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Err(e) = result {
        emit_progress(
            &app,
            &InstallProgress {
                instance_id: instance_id.clone(),
                version: Some(version.clone()),
                phase: "error".into(),
                line: Some(e.clone()),
            },
        );
        return Err(e);
    }

    // 校验并回写实例元数据
    let installed = read_installed_version(&instance_id)?;
    let mut meta = instance::load_meta(&dir)?;
    meta.dsh_version = Some(installed.clone());
    instance::save_meta(&dir, &meta)?;

    emit_progress(
        &app,
        &InstallProgress {
            instance_id: instance_id.clone(),
            version: Some(installed.clone()),
            phase: "done".into(),
            line: Some(format!("已安装 {DSH_PACKAGE}@{installed}")),
        },
    );
    Ok(installed)
}

#[tauri::command]
pub fn get_installed_version(instance_id: String) -> Result<Option<String>, String> {
    if !installed_pkg_json(&instance_id)?.is_file() {
        return Ok(None);
    }
    Ok(Some(read_installed_version(&instance_id)?))
}

/// 卸载实例内的 dsh（清空 pkg 下依赖并重置版本记录）
#[tauri::command]
pub fn uninstall_version(instance_id: String) -> Result<(), String> {
    let dir = instance::instance_dir(&instance_id)?;
    if !dir.is_dir() {
        return Err(format!("实例 {instance_id} 不存在"));
    }
    let pkg = pkg_dir(&instance_id)?;
    if pkg.exists() {
        fs::remove_dir_all(&pkg).map_err(|e| format!("删除失败: {e}"))?;
    }
    let _ = write_pkg_manifest(&instance_id, "*")?;

    let mut meta = instance::load_meta(&dir)?;
    meta.dsh_version = None;
    instance::save_meta(&dir, &meta)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::classify_channel;

    #[test]
    fn channel_classification() {
        assert_eq!(classify_channel("0.1.5-rc.2", Some("0.1.5-rc.2")), "latest");
        assert_eq!(classify_channel("0.1.5-rc.2", Some("0.1.6-alpha.2")), "rc");
        assert_eq!(classify_channel("0.1.6-alpha.2", None), "alpha");
        assert_eq!(classify_channel("1.0.0", None), "stable");
    }

    #[test]
    fn semver_sorts_prereleases() {
        let mut vs = vec!["0.1.6-alpha.2", "0.1.5-rc.2", "0.0.1-rc.1", "0.1.2-rc.1"];
        vs.sort_by(|a, b| {
            semver::Version::parse(b).unwrap().cmp(&semver::Version::parse(a).unwrap())
        });
        assert_eq!(vs[0], "0.1.6-alpha.2");
        assert_eq!(vs[1], "0.1.5-rc.2");
        assert_eq!(vs[3], "0.0.1-rc.1");
    }
}

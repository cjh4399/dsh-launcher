use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 一次运行中的 dsh 进程句柄。child 本体被 waiter 线程持有，
/// 停止时只需要 pid（taskkill 树杀）。
pub struct ChildHandle {
    pub pid: u32,
    pub port: u16,
    pub started_at: i64,
    pub boot_url: Mutex<Option<String>>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningInfo {
    pub instance_id: String,
    pub pid: u32,
    pub port: u16,
    pub started_at: i64,
    pub boot_url: Option<String>,
}

pub struct AppState {
    pub children: Mutex<HashMap<String, ChildHandle>>,
    /// 每个实例的日志环形缓冲，重进日志页时可回放
    pub logs: Mutex<HashMap<String, VecDeque<String>>>,
}

const LOG_BUFFER_CAP: usize = 4000;

impl AppState {
    pub fn new() -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
        }
    }

    pub fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    pub fn push_log(&self, id: &str, line: &str) {
        let mut logs = self.logs.lock().unwrap();
        let buf = logs.entry(id.to_string()).or_default();
        if buf.len() >= LOG_BUFFER_CAP {
            buf.pop_front();
        }
        buf.push_back(line.to_string());
    }

    pub fn log_buffer(&self, id: &str) -> Vec<String> {
        self.logs
            .lock()
            .unwrap()
            .get(id)
            .map(|b| b.iter().cloned().collect())
            .unwrap_or_default()
    }
}

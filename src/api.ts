import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface InstanceMeta {
  id: string;
  name: string;
  icon: string;
  color: string | null;
  port: number;
  dshVersion: string | null;
  createdAt: number;
  lastLaunchedAt: number | null;
}

export interface Settings {
  dataRoot: string | null;
  registry: string | null;
  theme: string; // "dark" | "light"
  autoOpenBrowser: boolean;
}

export interface NodeInfo {
  path: string;
  version: string;
}

export interface VersionInfo {
  version: string;
  channel: string; // latest | rc | alpha | stable
  publishedAt: number | null;
}

export interface RunningInfo {
  instanceId: string;
  pid: number;
  port: number;
  startedAt: number;
  bootUrl: string | null;
}

export interface InstallProgress {
  instanceId: string;
  version: string | null;
  phase: "resolving" | "installing" | "done" | "error";
  line: string | null;
}

export interface InstanceLogLine {
  instanceId: string;
  stream: string;
  line: string;
}

export interface InstanceStatus {
  instanceId: string;
  status: "starting" | "ready" | "stopped";
  code: number | null;
  url: string | null;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<Settings>("save_settings", { settings }),
  detectNode: () => invoke<NodeInfo>("detect_node_cmd"),

  listInstances: () => invoke<InstanceMeta[]>("list_instances"),
  createInstance: (name: string, icon?: string, port?: number) =>
    invoke<InstanceMeta>("create_instance", { name, icon, port }),
  updateInstance: (
    id: string,
    patch: { name?: string; icon?: string; color?: string; port?: number },
  ) => invoke<InstanceMeta>("update_instance", { id, ...patch }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  duplicateInstance: (id: string, newName: string) =>
    invoke<InstanceMeta>("duplicate_instance", { id, newName }),
  openInstanceDir: (id: string) => invoke<void>("open_instance_dir", { id }),
  openDataDir: () => invoke<void>("open_data_dir"),
  getLogBuffer: (id: string) => invoke<string[]>("get_instance_log_buffer", { id }),
  openUrl: (url: string) => invoke<void>("open_url_in_browser", { url }),

  fetchVersions: () => invoke<VersionInfo[]>("fetch_versions"),
  installVersion: (instanceId: string, version: string) =>
    invoke<string>("install_version", { instanceId, version }),
  getInstalledVersion: (instanceId: string) =>
    invoke<string | null>("get_installed_version", { instanceId }),
  uninstallVersion: (instanceId: string) =>
    invoke<void>("uninstall_version", { instanceId }),

  startInstance: (id: string) => invoke<RunningInfo>("start_instance", { id }),
  stopInstance: (id: string) => invoke<void>("stop_instance", { id }),
  getRunning: () => invoke<RunningInfo[]>("get_running"),
};

export function onInstanceLog(handler: (l: InstanceLogLine) => void): Promise<UnlistenFn> {
  return listen<InstanceLogLine>("instance-log", (e) => handler(e.payload));
}

export function onInstanceStatus(handler: (s: InstanceStatus) => void): Promise<UnlistenFn> {
  return listen<InstanceStatus>("instance-status", (e) => handler(e.payload));
}

export function onInstallProgress(handler: (p: InstallProgress) => void): Promise<UnlistenFn> {
  return listen<InstallProgress>("install-progress", (e) => handler(e.payload));
}

export function fmtTime(ms: number | null | undefined): string {
  if (!ms) return "—";
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

export const CHANNEL_LABEL: Record<string, string> = {
  latest: "推荐",
  rc: "预览",
  alpha: "实验",
  stable: "正式",
};

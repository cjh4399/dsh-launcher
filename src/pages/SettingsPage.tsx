import { useEffect, useState } from "react";
import { api, type NodeInfo } from "../api";
import { Icons } from "../components/ui";
import { useApp } from "../store";

const REGISTRY_PRESETS = [
  { label: "npmmirror（国内镜像，推荐）", value: "https://registry.npmmirror.com" },
  { label: "npmjs（官方源）", value: "https://registry.npmjs.org" },
];

export default function SettingsPage() {
  const { settings, setSettings, toast } = useApp();
  const [node, setNode] = useState<NodeInfo | null>(null);
  const [dataRoot, setDataRoot] = useState("");
  const [registry, setRegistry] = useState("");
  const [autoOpen, setAutoOpen] = useState(true);
  const [theme, setTheme] = useState("dark");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setDataRoot(settings.dataRoot ?? "");
    setRegistry(
      settings.registry ??
        "https://registry.npmmirror.com",
    );
    setAutoOpen(settings.autoOpenBrowser);
    setTheme(settings.theme || "dark");
    api.detectNode().then(setNode).catch(() => setNode(null));
  }, [settings]);

  async function save() {
    setBusy(true);
    try {
      const s = await api.saveSettings({
        dataRoot: dataRoot.trim() || null,
        registry: registry.trim() || null,
        theme,
        autoOpenBrowser: autoOpen,
      });
      setSettings(s);
      document.documentElement.dataset.theme = theme;
      toast("ok", "设置已保存");
      api.detectNode().then(setNode).catch(() => setNode(null));
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!settings) return null;

  return (
    <div>
      <h1 className="page-title">全局设置</h1>
      <p className="page-desc">影响所有实例的全局选项，保存在启动器目录下的 dsh-launcher.json。</p>

      <div className="card" style={{ maxWidth: 640 }}>
        <div className="field">
          <label>Node.js（自动从 PATH 检测）</label>
          <div className="hint">
            {node ? (
              <>
                检测到：<span className="mono">{node.path}</span>（{node.version}）
              </>
            ) : (
              <span style={{ color: "var(--err)" }}>
                未检测到 Node.js —— 请安装 Node.js（≥ 20.6）并确保其在 PATH 中
              </span>
            )}
          </div>
        </div>

        <div className="field">
          <label>npm registry（版本列表与安装来源）</label>
          <select value={registry} onChange={(e) => setRegistry(e.target.value)}>
            {REGISTRY_PRESETS.map((r) => (
              <option key={r.value} value={r.value}>
                {r.label}
              </option>
            ))}
            {!REGISTRY_PRESETS.some((r) => r.value === registry) && registry && (
              <option value={registry}>{registry}（自定义）</option>
            )}
          </select>
        </div>

        <div className="field">
          <label>数据目录（实例存放位置，留空 = 启动器目录下 dsh-data）</label>
          <input
            className="mono"
            style={{ fontSize: 12.5 }}
            value={dataRoot}
            onChange={(e) => setDataRoot(e.target.value)}
            placeholder="例如 D:\\dsh-data"
          />
          <div className="hint row">
            <span className="grow">
              更改后新实例将放到新目录；已有实例不会自动迁移。
            </span>
            <button className="btn" onClick={() => api.openDataDir()}>
              <Icons.Folder size={15} /> 打开当前目录
            </button>
          </div>
        </div>

        <div className="field">
          <label>外观</label>
          <div className="row">
            {[
              { v: "dark", label: "深色" },
              { v: "light", label: "浅色" },
            ].map((t) => (
              <button
                key={t.v}
                className={`btn${theme === t.v ? " btn-primary" : ""}`}
                onClick={() => {
                  setTheme(t.v);
                  document.documentElement.dataset.theme = t.v;
                }}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>

        <div className="field">
          <label>启动后自动打开 Web UI</label>
          <div
            className={`switch${autoOpen ? " on" : ""}`}
            onClick={() => setAutoOpen((v) => !v)}
          />
          <div className="hint">
            dsh 启动完成后，自动用系统浏览器打开带信任 token 的实例地址。
          </div>
        </div>

        <div className="row" style={{ justifyContent: "flex-end", marginTop: 20 }}>
          <button className="btn btn-primary" onClick={save} disabled={busy}>
            保存设置
          </button>
        </div>
      </div>
    </div>
  );
}

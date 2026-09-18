import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  api,
  fmtTime,
  onInstanceLog,
  onInstanceStatus,
  type InstanceMeta,
} from "../api";
import { Icons, Spinner, StatusDot } from "../components/ui";
import { useApp } from "../store";

function SettingsTab({
  meta,
  onSaved,
}: {
  meta: InstanceMeta;
  onSaved: () => void;
}) {
  const { toast } = useApp();
  const [name, setName] = useState(meta.name);
  const [icon, setIcon] = useState(meta.icon);
  const [port, setPort] = useState(String(meta.port));
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setName(meta.name);
    setIcon(meta.icon);
    setPort(String(meta.port));
  }, [meta]);

  async function save() {
    setBusy(true);
    try {
      await api.updateInstance(meta.id, {
        name: name.trim(),
        icon,
        port: port ? Number(port) : undefined,
      });
      toast("ok", "实例设置已保存");
      onSaved();
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="card" style={{ maxWidth: 560 }}>
      <div className="field">
        <label>实例名称</label>
        <input value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="field">
        <label>图标</label>
        <div className="row" style={{ flexWrap: "wrap", gap: 6 }}>
          {["🚀", "🤖", "🧠", "⚡", "🌊", "🔥", "🌙", "🛠️", "📦", "🧪"].map(
            (c) => (
              <button
                key={c}
                className={`btn${c === icon ? " btn-primary" : " btn-ghost"}`}
                style={{ fontSize: 18, padding: "6px 10px" }}
                onClick={() => setIcon(c)}
              >
                {c}
              </button>
            ),
          )}
        </div>
      </div>
      <div className="field">
        <label>端口</label>
        <input
          value={port}
          onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))}
        />
        <div className="hint">
          首选端口被占用时，启动会自动顺延到下一个空闲端口（实际端口见运行状态）。
        </div>
      </div>
      <div className="field">
        <label>实例目录</label>
        <div className="row">
          <span className="dim mono" style={{ fontSize: 12 }}>
            instances/{meta.id}/
          </span>
          <button
            className="btn"
            style={{ marginLeft: "auto" }}
            onClick={() => api.openInstanceDir(meta.id)}
          >
            <Icons.Folder size={15} /> 打开
          </button>
        </div>
        <div className="hint">
          pkg/ = dsh 安装 · home/ = DSH_HOME（profiles、凭据、配置）· workspace/ =
          agent 工作目录
        </div>
      </div>
      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button className="btn btn-primary" onClick={save} disabled={busy}>
          保存设置
        </button>
      </div>
    </div>
  );
}

function LogTab({ id }: { id: string }) {
  const { running, toast } = useApp();
  const [lines, setLines] = useState<string[]>([]);
  const [autoscroll, setAutoscroll] = useState(true);
  const boxRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.getLogBuffer(id).then((buf) => setLines(buf));
    const unLog = onInstanceLog((l) => {
      if (l.instanceId !== id) return;
      setLines((prev) => [...prev.slice(-4000), l.line]);
    });
    const unStatus = onInstanceStatus(() => {});
    return () => {
      unLog.then((f) => f());
      unStatus.then((f) => f());
    };
  }, [id]);

  useEffect(() => {
    if (autoscroll && boxRef.current) {
      boxRef.current.scrollTop = boxRef.current.scrollHeight;
    }
  }, [lines, autoscroll]);

  function exportLog() {
    const blob = new Blob([lines.join("\n")], { type: "text/plain" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `dsh-${id.slice(0, 20)}.log`;
    a.click();
    URL.revokeObjectURL(a.href);
    toast("ok", "日志已导出");
  }

  const r = running[id];

  return (
    <div>
      <div className="log-toolbar">
        <span className="row" style={{ gap: 7 }}>
          <StatusDot state={!r ? "stopped" : r.bootUrl ? "ready" : "starting"} />
          {!r
            ? "未运行"
            : r.bootUrl
              ? `运行中 · ${r.bootUrl}`
              : "启动中…"}
        </span>
        <div className="grow" />
        <button
          className={`btn${autoscroll ? " btn-primary" : ""}`}
          style={{ padding: "4px 12px" }}
          onClick={() => setAutoscroll((v) => !v)}
        >
          自动滚动
        </button>
        <button
          className="btn"
          style={{ padding: "4px 12px" }}
          onClick={() => setLines([])}
        >
          清屏
        </button>
        <button
          className="btn"
          style={{ padding: "4px 12px" }}
          onClick={exportLog}
        >
          导出
        </button>
      </div>
      <div className="log-panel" ref={boxRef}>
        {lines.length === 0 ? (
          <div className="dim" style={{ fontFamily: "var(--font)" }}>
            （暂无日志 —— 启动实例后，stdout/stderr 会实时显示在这里）
          </div>
        ) : (
          lines.map((l, i) => (
            <div
              key={i}
              className={`log-line${
                l.startsWith("[launcher]")
                  ? " system"
                  : l.includes("[stderr]") || l.toLowerCase().includes("error")
                    ? " stderr"
                    : ""
              }`}
            >
              {l}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default function InstanceDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { instances, refreshInstances, running } = useApp();
  const [tab, setTab] = useState<"settings" | "logs">("settings");
  const meta = instances.find((i) => i.id === id);

  const reload = useCallback(async () => {
    await refreshInstances();
  }, [refreshInstances]);

  useEffect(() => {
    if (!meta && instances.length > 0) {
      navigate("/instances", { replace: true });
    }
  }, [meta, instances, navigate]);

  if (!meta) {
    return (
      <div className="empty">
        <Spinner size={24} />
        <div className="dim">加载实例信息…</div>
      </div>
    );
  }

  const r = running[meta.id];

  return (
    <div>
      <div className="row">
        <div className="grow">
          <h1 className="page-title">
            {meta.icon} {meta.name}
          </h1>
          <p className="page-desc">
            {meta.dshVersion ? `dsh ${meta.dshVersion}` : "未安装 dsh"} · 端口{" "}
            {r?.port || meta.port} · 创建于 {fmtTime(meta.createdAt)}
          </p>
        </div>
        {r?.bootUrl && (
          <button
            className="btn btn-primary"
            onClick={() => api.openUrl(r.bootUrl!)}
          >
            <Icons.External size={15} /> 打开 Web UI
          </button>
        )}
      </div>

      <div className="tabs">
        <button
          className={`tab${tab === "settings" ? " active" : ""}`}
          onClick={() => setTab("settings")}
        >
          实例设置
        </button>
        <button
          className={`tab${tab === "logs" ? " active" : ""}`}
          onClick={() => setTab("logs")}
        >
          运行日志
        </button>
      </div>

      {tab === "settings" ? (
        <SettingsTab meta={meta} onSaved={reload} />
      ) : (
        <LogTab id={meta.id} />
      )}
    </div>
  );
}

import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, fmtTime } from "../api";
import { Icons, Spinner, StatusDot } from "../components/ui";
import { useApp } from "../store";

export default function Home() {
  const { instances, running, toast } = useApp();
  const navigate = useNavigate();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!selectedId && instances.length > 0) {
      setSelectedId(instances[0].id);
    }
  }, [instances, selectedId]);

  const selected = useMemo(
    () => instances.find((i) => i.id === selectedId) ?? null,
    [instances, selectedId],
  );
  const runningInfo = selected ? running[selected.id] : undefined;
  const status: "stopped" | "starting" | "ready" = !runningInfo
    ? "stopped"
    : runningInfo.bootUrl
      ? "ready"
      : "starting";

  async function launch() {
    if (!selected) return;
    setBusy(true);
    try {
      await api.startInstance(selected.id);
      toast("info", `正在启动「${selected.name}」…`);
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  async function stop() {
    if (!selected) return;
    setBusy(true);
    try {
      await api.stopInstance(selected.id);
      toast("ok", "实例已停止");
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  async function openWeb() {
    if (!runningInfo) return;
    try {
      const url =
        runningInfo.bootUrl ?? `http://127.0.0.1:${runningInfo.port}/`;
      await api.openUrl(url);
    } catch (e) {
      toast("error", String(e));
    }
  }

  return (
    <div>
      <h1 className="page-title">主页</h1>
      <p className="page-desc">选择一个实例，然后启动它。</p>

      {instances.length === 0 ? (
        <div className="empty">
          <div className="big-icon">🚀</div>
          <div>
            <div style={{ fontWeight: 700, color: "var(--text)", marginBottom: 6 }}>
              还没有实例
            </div>
            <div className="dim">
              实例 = 一个完全隔离的 dsh 运行环境（独立版本 / 端口 / 配置 / 工作区）
            </div>
          </div>
          <button className="btn btn-primary" onClick={() => navigate("/instances")}>
            创建第一个实例
          </button>
        </div>
      ) : (
        <div className="home">
          <div className="home-list">
            {instances.map((i) => {
              const r = running[i.id];
              return (
                <button
                  key={i.id}
                  className={`instance-card${i.id === selectedId ? " selected" : ""}`}
                  onClick={() => setSelectedId(i.id)}
                >
                  <div className="avatar">{i.icon}</div>
                  <div className="meta">
                    <div className="name">
                      {i.name}
                      <StatusDot
                        state={!r ? "stopped" : r.bootUrl ? "ready" : "starting"}
                      />
                    </div>
                    <div className="sub">
                      {i.dshVersion ? `dsh ${i.dshVersion}` : "未安装 dsh"}
                      {" · "}
                      端口 {i.port}
                    </div>
                  </div>
                </button>
              );
            })}
            <button
              className="instance-card"
              style={{ justifyContent: "center", color: "var(--text-dim)" }}
              onClick={() => navigate("/instances")}
            >
              ＋ 新建实例
            </button>
          </div>

          <div className="hero">
            <h2 className="hero-title">{selected ? selected.name : "—"}</h2>
            <p className="hero-sub">
              {selected
                ? `${selected.dshVersion ? `dsh ${selected.dshVersion}` : "尚未安装 dsh"} · 端口 ${selected.port} · 上次启动 ${fmtTime(selected.lastLaunchedAt)}`
                : ""}
            </p>

            {status === "stopped" ? (
              <button
                className="launch-btn"
                onClick={launch}
                disabled={busy || !selected}
              >
                {busy ? <Spinner size={26} /> : <Icons.Play size={44} />}
                <span className="big">启动</span>
              </button>
            ) : (
              <button className="launch-btn stop" onClick={stop} disabled={busy}>
                <Icons.Stop size={40} />
                <span className="big">停止</span>
              </button>
            )}

            <div className="hero-status">
              <StatusDot state={status} />
              {status === "stopped" &&
                (selected?.dshVersion
                  ? "已停止"
                  : "已停止（尚未安装 dsh，点击启动会提示；请先到「下载」页安装）")}
              {status === "starting" && "启动中，正在等待端口就绪…"}
              {status === "ready" && `运行中 · PID ${runningInfo?.pid || "—"} · 端口 ${runningInfo?.port}`}
            </div>

            <div className="hero-actions">
              <button
                className="btn"
                onClick={openWeb}
                disabled={status !== "ready"}
              >
                <Icons.External size={16} /> 打开 Web UI
              </button>
              <button
                className="btn"
                onClick={() =>
                  selected &&
                  navigate(`/instance/${selected.id}`)
                }
                disabled={!selected}
              >
                <Icons.Terminal size={16} /> 设置与日志
              </button>
              <button
                className="btn"
                onClick={() => navigate("/download")}
              >
                <Icons.Download size={16} /> 下载 dsh
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

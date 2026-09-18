import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, fmtTime } from "../api";
import { Icons, Modal, StatusDot } from "../components/ui";
import { useApp } from "../store";

const ICON_CHOICES = ["🚀", "🤖", "🧠", "⚡", "🌊", "🔥", "🌙", "🛠️", "📦", "🧪"];

function CreateModal({ onClose }: { onClose: () => void }) {
  const { refreshInstances, toast } = useApp();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [icon, setIcon] = useState(ICON_CHOICES[0]);
  const [port, setPort] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit() {
    if (!name.trim()) {
      toast("error", "请输入实例名称");
      return;
    }
    setBusy(true);
    try {
      const meta = await api.createInstance(
        name.trim(),
        icon,
        port ? Number(port) : undefined,
      );
      toast("ok", `实例「${meta.name}」已创建（端口 ${meta.port}）`);
      await refreshInstances();
      onClose();
      navigate(`/instance/${meta.id}`);
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title="创建新实例" onClose={onClose}>
      <div className="field">
        <label>实例名称</label>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="例如：日常主力 / 测试预览版"
          autoFocus
        />
        <div className="hint">
          每个实例是完全隔离的：独立的 dsh 版本、独立端口、独立配置与工作区。
        </div>
      </div>
      <div className="field">
        <label>图标</label>
        <div className="row" style={{ flexWrap: "wrap", gap: 6 }}>
          {ICON_CHOICES.map((c) => (
            <button
              key={c}
              className={`btn${c === icon ? " btn-primary" : " btn-ghost"}`}
              style={{ fontSize: 18, padding: "6px 10px" }}
              onClick={() => setIcon(c)}
            >
              {c}
            </button>
          ))}
        </div>
      </div>
      <div className="field">
        <label>端口（留空自动分配）</label>
        <input
          value={port}
          onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))}
          placeholder="3080"
        />
        <div className="hint">端口被占用时会自动向后顺延。</div>
      </div>
      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button className="btn" onClick={onClose}>
          取消
        </button>
        <button className="btn btn-primary" onClick={submit} disabled={busy}>
          创建
        </button>
      </div>
    </Modal>
  );
}

export default function Instances() {
  const { instances, running, refreshInstances, toast } = useApp();
  const navigate = useNavigate();
  const [showCreate, setShowCreate] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const target = instances.find((i) => i.id === confirmDelete);

  async function act(fn: () => Promise<unknown>, okMsg?: string) {
    setBusy(true);
    try {
      await fn();
      if (okMsg) toast("ok", okMsg);
      await refreshInstances();
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <div className="row">
        <div className="grow">
          <h1 className="page-title">实例管理</h1>
          <p className="page-desc">
            共 {instances.length} 个实例 · 运行中{" "}
            {Object.keys(running).length} 个
          </p>
        </div>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
          ＋ 新建实例
        </button>
      </div>

      {instances.length === 0 ? (
        <div className="empty">
          <div className="big-icon">🗂️</div>
          <div className="dim">还没有实例，点击右上角「新建实例」开始。</div>
        </div>
      ) : (
        <div className="card" style={{ padding: 6 }}>
          <table className="table">
            <thead>
              <tr>
                <th>实例</th>
                <th>dsh 版本</th>
                <th>端口</th>
                <th>状态</th>
                <th>创建时间</th>
                <th style={{ width: 250 }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {instances.map((i) => {
                const r = running[i.id];
                const st = !r ? "stopped" : r.bootUrl ? "ready" : "starting";
                return (
                  <tr key={i.id}>
                    <td>
                      <div className="row">
                        <span style={{ fontSize: 20 }}>{i.icon}</span>
                        <div>
                          <div style={{ fontWeight: 600 }}>{i.name}</div>
                          <div
                            className="dim mono"
                            style={{ fontSize: 11 }}
                          >
                            {i.id}
                          </div>
                        </div>
                      </div>
                    </td>
                    <td className="mono">{i.dshVersion ?? "—"}</td>
                    <td className="mono">{i.port}</td>
                    <td>
                      <span className="row" style={{ gap: 7 }}>
                        <StatusDot state={st} />
                        {st === "stopped"
                          ? "已停止"
                          : st === "starting"
                            ? "启动中"
                            : `运行中`}
                      </span>
                    </td>
                    <td className="dim">{fmtTime(i.createdAt)}</td>
                    <td>
                      <div className="row" style={{ gap: 4, flexWrap: "wrap" }}>
                        {st === "stopped" ? (
                          <button
                            className="btn btn-primary"
                            style={{ padding: "5px 12px" }}
                            disabled={busy}
                            title={i.dshVersion ? "" : "该实例尚未安装 dsh，启动会提示先安装"}
                            onClick={() =>
                              act(
                                () => api.startInstance(i.id),
                                `「${i.name}」启动中…`,
                              )
                            }
                          >
                            启动
                          </button>
                        ) : (
                          <>
                            <button
                              className="btn btn-danger"
                              style={{ padding: "5px 12px" }}
                              disabled={busy}
                              onClick={() => act(() => api.stopInstance(i.id), "已停止")}
                            >
                              停止
                            </button>
                            <button
                              className="btn"
                              style={{ padding: "5px 12px" }}
                              disabled={st !== "ready"}
                              onClick={() =>
                                act(async () => {
                                  const info = running[i.id];
                                  await api.openUrl(
                                    info?.bootUrl ??
                                      `http://127.0.0.1:${info?.port ?? i.port}/`,
                                  );
                                })
                              }
                            >
                              打开
                            </button>
                          </>
                        )}
                        <button
                          className="icon-btn"
                          title="实例目录"
                          onClick={() => act(() => api.openInstanceDir(i.id))}
                        >
                          <Icons.Folder size={16} />
                        </button>
                        <button
                          className="icon-btn"
                          title="设置与日志"
                          onClick={() => navigate(`/instance/${i.id}`)}
                        >
                          <Icons.Cog size={16} />
                        </button>
                        <button
                          className="icon-btn"
                          title="复制实例"
                          disabled={busy || st !== "stopped"}
                          onClick={() => {
                            const newName = window.prompt(
                              `输入新实例名称（复制自「${i.name}」，配置与工作区会被拷贝，dsh 版本需重新安装）`,
                              `${i.name} - 副本`,
                            );
                            if (newName)
                              act(
                                () => api.duplicateInstance(i.id, newName),
                                "复制完成",
                              );
                          }}
                        >
                          <Icons.Copy size={16} />
                        </button>
                        <button
                          className="icon-btn"
                          title="删除实例"
                          disabled={busy || st !== "stopped"}
                          onClick={() => setConfirmDelete(i.id)}
                        >
                          <Icons.Trash size={16} />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {showCreate && <CreateModal onClose={() => setShowCreate(false)} />}

      {confirmDelete && target && (
        <Modal title="删除实例" onClose={() => setConfirmDelete(null)}>
          <div style={{ lineHeight: 1.8, marginBottom: 18 }}>
            确定要删除实例「<b>{target.name}</b>」吗？
            <br />
            <span className="dim" style={{ fontSize: 12.5 }}>
              将一并删除它的 dsh 安装、配置、凭据与工作区文件，此操作不可恢复。
            </span>
          </div>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="btn" onClick={() => setConfirmDelete(null)}>
              取消
            </button>
            <button
              className="btn btn-danger"
              disabled={busy}
              onClick={() =>
                act(async () => {
                  await api.deleteInstance(target.id);
                  setConfirmDelete(null);
                }, "实例已删除")
              }
            >
              确认删除
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}

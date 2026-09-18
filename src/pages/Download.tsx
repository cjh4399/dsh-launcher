import { useCallback, useEffect, useRef, useState } from "react";
import {
  api,
  CHANNEL_LABEL,
  fmtTime,
  onInstallProgress,
  type InstallProgress,
  type VersionInfo,
} from "../api";
import { ChannelBadge, Icons, Spinner } from "../components/ui";
import { useApp } from "../store";

type Filter = "all" | "latest" | "rc" | "alpha";

interface InstallState {
  version: string;
  phase: InstallProgress["phase"];
  lines: string[];
}

export default function Download() {
  const { instances, refreshInstances, toast } = useApp();
  const [versions, setVersions] = useState<VersionInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [targetId, setTargetId] = useState<string>("");
  const [installing, setInstalling] = useState<Record<string, InstallState>>({});
  const unInstall = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (!targetId && instances.length > 0) setTargetId(instances[0].id);
  }, [instances, targetId]);

  const load = useCallback(async () => {
    setError(null);
    setVersions(null);
    try {
      setVersions(await api.fetchVersions());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    load();
    onInstallProgress((p) => {
      setInstalling((m) => {
        const cur = m[p.instanceId];
        if (p.phase === "done" || p.phase === "error") {
          const rest = { ...m };
          delete rest[p.instanceId];
          return rest;
        }
        return {
          ...m,
          [p.instanceId]: {
            version: p.version ?? cur?.version ?? "",
            phase: p.phase,
            lines: [...(cur?.lines ?? []), p.line ?? ""].slice(-6),
          },
        };
      });
      // 完成事件到达时立刻对账实例列表：即使 invoke 返回链路有问题，
      // 实例卡片的版本号与启动按钮也能及时更新
      if (p.phase === "done") {
        refreshInstances();
      } else if (p.phase === "error") {
        toast("error", `安装失败：${p.line ?? "未知错误"}`);
        refreshInstances();
      }
    }).then((un) => (unInstall.current = un));
    return () => unInstall.current?.();
  }, [load, refreshInstances, toast]);

  async function install(v: string) {
    if (!targetId) {
      toast("error", "请先创建一个实例，再安装 dsh");
      return;
    }
    const target = instances.find((i) => i.id === targetId);
    setInstalling((m) => ({
      ...m,
      [targetId]: { version: v, phase: "resolving", lines: [] },
    }));
    try {
      const got = await api.installVersion(targetId, v);
      toast("ok", `已为「${target?.name}」安装 dsh ${got}`);
      await refreshInstances();
    } catch (e) {
      toast("error", String(e));
    } finally {
      setInstalling((m) => {
        const rest = { ...m };
        delete rest[targetId];
        return rest;
      });
    }
  }

  async function uninstall(instanceId: string) {
    const target = instances.find((i) => i.id === instanceId);
    if (!window.confirm(`确定卸载「${target?.name}」中的 dsh 吗？（配置与工作区保留）`))
      return;
    try {
      await api.uninstallVersion(instanceId);
      toast("ok", "已卸载");
      await refreshInstances();
    } catch (e) {
      toast("error", String(e));
    }
  }

  const shown = (versions ?? []).filter((v) =>
    filter === "all" ? true : v.channel === filter,
  );

  return (
    <div>
      <div className="row">
        <div className="grow">
          <h1 className="page-title">下载 dsh</h1>
          <p className="page-desc">
            从 npm registry 获取 @deepseek-ai/dsh 的全部版本，安装到指定实例（按实例隔离）。
          </p>
        </div>
        <div className="field" style={{ marginBottom: 0, minWidth: 220 }}>
          <select
            value={targetId}
            onChange={(e) => setTargetId(e.target.value)}
          >
            {instances.length === 0 && <option value="">（还没有实例）</option>}
            {instances.map((i) => (
              <option key={i.id} value={i.id}>
                {i.icon} {i.name}
                {i.dshVersion ? `（当前 ${i.dshVersion}）` : "（未安装）"}
              </option>
            ))}
          </select>
        </div>
        <button className="btn" onClick={load} disabled={!versions && !error}>
          <Icons.Refresh size={16} /> 刷新
        </button>
      </div>

      {instances.length === 0 && (
        <div className="empty">
          <div className="big-icon">📦</div>
          <div className="dim">先创建一个实例，再回来安装 dsh。</div>
        </div>
      )}

      {error && (
        <div className="card" style={{ borderColor: "var(--err)" }}>
          <b>获取版本列表失败</b>
          <div className="dim mono" style={{ fontSize: 12, marginTop: 6 }}>
            {error}
          </div>
          <div className="mt">
            可在「全局设置」里切换 npm registry（npmmirror / npmjs）后重试。
          </div>
        </div>
      )}

      {versions && (
        <>
          <div className="tabs">
            {(["all", "latest", "rc", "alpha"] as Filter[]).map((f) => (
              <button
                key={f}
                className={`tab${filter === f ? " active" : ""}`}
                onClick={() => setFilter(f)}
              >
                {f === "all" ? "全部" : CHANNEL_LABEL[f]}
              </button>
            ))}
          </div>

          {targetId && installing[targetId] && (
            <div className="card" style={{ marginBottom: 14 }}>
              <div className="row">
                {installing[targetId].phase === "resolving" ? (
                  <Spinner />
                ) : (
                  <Spinner />
                )}
                <b>
                  正在安装 dsh {installing[targetId].version} …
                </b>
              </div>
              <div className="progress indeterminate mt" style={{ marginBottom: 10 }}>
                <div className="bar" />
              </div>
              {installing[targetId].lines.map((l, idx) => (
                <div key={idx} className="line">
                  {l}
                </div>
              ))}
            </div>
          )}

          <div className="card" style={{ padding: 6 }}>
            <table className="table">
              <thead>
                <tr>
                  <th>版本</th>
                  <th>通道</th>
                  <th>发布时间</th>
                  <th style={{ width: 200 }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {shown.map((v) => {
                  const installedHere = instances.some(
                    (i) => i.id === targetId && i.dshVersion === v.version,
                  );
                  const busyHere = targetId && !!installing[targetId];
                  return (
                    <tr key={v.version}>
                      <td className="mono" style={{ fontWeight: 600 }}>
                        {v.version}
                      </td>
                      <td>
                        <ChannelBadge channel={v.channel} />
                      </td>
                      <td className="dim">{fmtTime(v.publishedAt)}</td>
                      <td>
                        <div className="row">
                          <button
                            className={
                              installedHere ? "btn" : "btn btn-primary"
                            }
                            style={{ padding: "5px 14px" }}
                            disabled={!targetId || !!busyHere}
                            onClick={() => install(v.version)}
                          >
                            {installedHere
                              ? "重装"
                              : v.channel === "latest"
                                ? "安装"
                                : "安装此版本"}
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="mt">
            <div className="dim" style={{ fontSize: 12.5, lineHeight: 1.9 }}>
              通道说明：
              <ChannelBadge channel="latest" /> 当前 dist-tag 指向的推荐版本 ·
              <ChannelBadge channel="rc" /> 发布候选（相对稳定） ·
              <ChannelBadge channel="alpha" /> 实验版本（可能有破坏性变更）。
              <br />
              想卸载某个实例里的 dsh？在下方选择实例后点击「卸载」。
              <button
                className="btn btn-danger"
                style={{ marginLeft: 10, padding: "3px 12px" }}
                disabled={!targetId}
                onClick={() => uninstall(targetId)}
              >
                卸载所选实例的 dsh
              </button>
            </div>
          </div>
        </>
      )}

      {!versions && !error && (
        <div className="empty">
          <Spinner size={26} />
          <div className="dim">正在从 registry 获取版本列表…</div>
        </div>
      )}
    </div>
  );
}

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  api,
  onInstanceStatus,
  type InstanceMeta,
  type RunningInfo,
  type Settings,
} from "./api";

export interface Toast {
  id: number;
  kind: "info" | "ok" | "error";
  text: string;
}

interface AppState {
  settings: Settings | null;
  setSettings: (s: Settings) => void;
  instances: InstanceMeta[];
  refreshInstances: () => Promise<void>;
  running: Record<string, RunningInfo>;
  refreshRunning: () => Promise<void>;
  toasts: Toast[];
  toast: (kind: Toast["kind"], text: string) => void;
}

const Ctx = createContext<AppState | null>(null);

export function useApp(): AppState {
  const v = useContext(Ctx);
  if (!v) throw new Error("AppProvider missing");
  return v;
}

let toastSeq = 1;

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [instances, setInstances] = useState<InstanceMeta[]>([]);
  const [running, setRunning] = useState<Record<string, RunningInfo>>({});
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef<number[]>([]);

  const toast = useCallback((kind: Toast["kind"], text: string) => {
    const id = toastSeq++;
    setToasts((t) => [...t, { id, kind, text }]);
    // 错误停留更久，避免被忽略
    const ms = kind === "error" ? 9000 : 3600;
    const timer = window.setTimeout(() => {
      setToasts((t) => t.filter((x) => x.id !== id));
    }, ms);
    timers.current.push(timer);
  }, []);

  const refreshInstances = useCallback(async () => {
    try {
      setInstances(await api.listInstances());
    } catch (e) {
      toast("error", String(e));
    }
  }, [toast]);

  const refreshRunning = useCallback(async () => {
    try {
      const list = await api.getRunning();
      const map: Record<string, RunningInfo> = {};
      for (const r of list) map[r.instanceId] = r;
      setRunning(map);
    } catch {
      /* 忽略 */
    }
  }, []);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      document.documentElement.dataset.theme = s.theme || "dark";
    });
    refreshInstances();
    refreshRunning();
    const un = onInstanceStatus((st) => {
      if (st.status === "stopped") {
        setRunning((m) => {
          const rest = { ...m };
          delete rest[st.instanceId];
          return rest;
        });
        refreshInstances();
      } else {
        setRunning((m) => {
          const prev = m[st.instanceId];
          return {
            ...m,
            [st.instanceId]: {
              instanceId: st.instanceId,
              pid: prev?.pid ?? 0,
              port: prev?.port ?? 0,
              startedAt: prev?.startedAt ?? Date.now(),
              bootUrl: st.url ?? prev?.bootUrl ?? null,
            },
          };
        });
      }
    });

    // 窗口获得焦点时对账一次：治愈任何漏刷新导致的过期状态
    // （例如安装完成事件丢失时，实例版本号/启动按钮会保持正确）
    const syncOnFocus = () => {
      refreshInstances();
      refreshRunning();
    };
    window.addEventListener("focus", syncOnFocus);
    document.addEventListener("visibilitychange", syncOnFocus);

    return () => {
      un.then((f) => f());
      window.removeEventListener("focus", syncOnFocus);
      document.removeEventListener("visibilitychange", syncOnFocus);
      timers.current.forEach((t) => window.clearTimeout(t));
    };
  }, [refreshInstances, refreshRunning]);

  return (
    <Ctx.Provider
      value={{
        settings,
        setSettings,
        instances,
        refreshInstances,
        running,
        refreshRunning,
        toasts,
        toast,
      }}
    >
      {children}
    </Ctx.Provider>
  );
}

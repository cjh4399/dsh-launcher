import { NavLink, Route, Routes } from "react-router-dom";
import { Icons } from "./components/ui";
import { useApp } from "./store";
import Home from "./pages/Home";
import Instances from "./pages/Instances";
import Download from "./pages/Download";
import InstanceDetail from "./pages/InstanceDetail";
import SettingsPage from "./pages/SettingsPage";

function Sidebar() {
  const item = ({ isActive }: { isActive: boolean }) =>
    `nav-item${isActive ? " active" : ""}`;
  return (
    <div className="sidebar">
      <div className="brand">
        <div className="brand-logo">DS</div>
        <div>
          <div className="brand-name">DSH Launcher</div>
          <div className="brand-sub">DeepSeek Harness 启动器</div>
        </div>
      </div>
      <NavLink to="/" end className={item}>
        <Icons.Home size={18} /> 主页
      </NavLink>
      <NavLink to="/instances" className={item}>
        <Icons.Grid size={18} /> 实例管理
      </NavLink>
      <NavLink to="/download" className={item}>
        <Icons.Download size={18} /> 下载
      </NavLink>
      <NavLink to="/settings" className={item}>
        <Icons.Cog size={18} /> 全局设置
      </NavLink>
      <div className="sidebar-footer">
        多实例 · 多版本 · 完全隔离
        <br />
        参考 PCL2 交互设计
      </div>
    </div>
  );
}

function Toasts() {
  const { toasts } = useApp();
  return (
    <div className="toasts">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}

export default function App() {
  return (
    <div className="app">
      <Sidebar />
      <div className="content">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/instances" element={<Instances />} />
          <Route path="/download" element={<Download />} />
          <Route path="/instance/:id" element={<InstanceDetail />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </div>
      <Toasts />
    </div>
  );
}

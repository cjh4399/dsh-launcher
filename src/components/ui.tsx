import React from "react";

export function Modal(props: {
  title: string;
  onClose: () => void;
  children: React.ReactNode;
  width?: number;
}) {
  return (
    <div className="modal-mask" onClick={props.onClose}>
      <div
        className="modal"
        style={{ width: props.width ?? 460 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-title">
          <span>{props.title}</span>
          <button className="icon-btn" onClick={props.onClose} title="关闭">
            ✕
          </button>
        </div>
        <div className="modal-body">{props.children}</div>
      </div>
    </div>
  );
}

export function Spinner({ size = 16 }: { size?: number }) {
  return <span className="spinner" style={{ width: size, height: size }} />;
}

export function StatusDot({ state }: { state: "stopped" | "starting" | "ready" }) {
  const cls =
    state === "ready" ? "dot dot-ready" : state === "starting" ? "dot dot-starting" : "dot";
  return <span className={cls} />;
}

export function ChannelBadge({ channel }: { channel: string }) {
  return <span className={`badge badge-${channel}`}>{channel}</span>;
}

/* ---- 图标（内联 SVG，线性风格） ---- */
function Svg(props: { d: string; size?: number }) {
  return (
    <svg
      width={props.size ?? 20}
      height={props.size ?? 20}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={props.d} />
    </svg>
  );
}

export const Icons = {
  Home: (p: { size?: number }) => (
    <Svg size={p.size} d="M3 10.5 12 3l9 7.5M5 9.5V21h14V9.5" />
  ),
  Grid: (p: { size?: number }) => (
    <Svg
      size={p.size}
      d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z"
    />
  ),
  Download: (p: { size?: number }) => (
    <Svg size={p.size} d="M12 3v12m0 0 4.5-4.5M12 15l-4.5-4.5M4 20h16" />
  ),
  Cog: (p: { size?: number }) => (
    <Svg
      size={p.size}
      d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.5-2-3.5-2.4 1a7.5 7.5 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7.5 7.5 0 0 0-2 1.2l-2.4-1-2 3.5 2 1.5a7.4 7.4 0 0 0 0 2.4l-2 1.5 2 3.5 2.4-1a7.5 7.5 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7.5 7.5 0 0 0 2-1.2l2.4 1 2-3.5-2-1.5c.1-.4.1-.8.1-1.2Z"
    />
  ),
  Play: (p: { size?: number }) => (
    <Svg size={p.size} d="M7 4.5v15l13-7.5-13-7.5Z" />
  ),
  Stop: (p: { size?: number }) => <Svg size={p.size} d="M6 6h12v12H6z" />,
  Folder: (p: { size?: number }) => (
    <Svg size={p.size} d="M3 6a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6Z" />
  ),
  Copy: (p: { size?: number }) => (
    <Svg size={p.size} d="M9 9h10v12H9zM5 15H3V3h12v2" />
  ),
  Trash: (p: { size?: number }) => (
    <Svg size={p.size} d="M4 7h16M9 7V4h6v3m-8 0 1 14h8l1-14" />
  ),
  Refresh: (p: { size?: number }) => (
    <Svg size={p.size} d="M20 11A8 8 0 0 0 6.3 6.3L4 8.5M4 5v4h4m-4 5a8 8 0 0 0 13.7 4.7L20 15.5m0 3.5v-4h-4" />
  ),
  External: (p: { size?: number }) => (
    <Svg size={p.size} d="M14 4h6v6m0-6L10 14M9 5H5v14h14v-4" />
  ),
  Terminal: (p: { size?: number }) => (
    <Svg size={p.size} d="M5 7l5 5-5 5m8 0h6" />
  ),
};

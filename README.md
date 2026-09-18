# DSH Launcher

一个参考 PCL2（Plain Craft Launcher 2）交互设计的 **DeepSeek Harness（dsh）启动器**，
用于管理 dsh 的**多实例**与**多版本**。

技术栈：Tauri 2 + React 19 + TypeScript。

## 功能总览

| 模块 | 说明 |
| --- | --- |
| 主页 | 实例选择卡片 + 大启动按钮（PCL2 风格），一键启动 / 停止 / 打开 Web UI |
| 实例管理 | 创建 / 复制 / 删除实例，打开实例目录，查看运行状态 |
| 下载 | 从 npm registry 拉取 `@deepseek-ai/dsh` 全部版本，按 通道（推荐 / 预览 / 实验）分类，安装到指定实例 |
| 实例设置 | 名称 / 图标 / 端口；实例目录说明 |
| 运行日志 | 实时 stdout/stderr、自动滚动、清空、导出 |
| 全局设置 | Node.js 路径检测、npm registry 切换（npmmirror / npmjs）、数据目录、深浅主题、自动打开浏览器 |

## 多实例多版本方案（版本隔离）

参考 PCL2 的「版本隔离」，每个实例是一个完全隔离的 dsh 运行环境：

```
<数据目录>/instances/<实例id>/
├── instance.json   实例元数据（名称 / 图标 / 端口 / 已装版本）
├── pkg/            该实例独立安装的 @deepseek-ai/dsh（npm --prefix 安装）
├── home/           作为 DSH_HOME 传给 dsh：profiles、凭据、配置都在这里
└── workspace/      dsh 进程的工作目录（agent 工作区）
```

- **多版本**：dsh 装在各自实例的 `pkg/` 里，实例 A 可用 `0.1.5-rc.2`、实例 B 用
  `0.1.6-alpha.2`，互不影响，可随时切换 / 回退 / 重装
- **多实例并行**：每实例绑定自己的端口（默认从 3080 自动避让），可同时运行多个实例
- **凭据隔离**：dsh 的凭据（`.credentials.yaml`）与 profiles 存放在各实例独立的
  `home/`（即 `DSH_HOME`）内，由 dsh Web UI 管理，实例之间天然隔离

## dsh 启动原理（实测）

启动器拉起实例的实际命令：

```
DSH_HOME=<实例>/home node <实例>/pkg/node_modules/@deepseek-ai/dsh/lib/bin.js web --port <端口> --no-open
```

要点（基于 `@deepseek-ai/dsh@0.1.5-rc.2` 实测）：

- `dsh web` 是 `--profile web` 的别名；`--port` / `--no-open` / `--host` /
  `--trusted-host` 为 web 应用参数
- `DSH_HOME` 环境变量决定 profile 与配置的根目录（默认 `~/.dsh`），
  启动器用它实现实例隔离
- 启动成功后 stdout 会打印带信任 token 的真实地址：
  `dsh web: http://127.0.0.1:<port>/?token=...` —— 启动器解析该行并自动打开浏览器
  （可在设置中关闭）
- 健康检查：启动器轮询 TCP 端口，端口就绪即认为实例可用（首字节 401 属正常，
  说明 Web UI 已在监听）
- 停止：`taskkill /F /T` 树杀整个进程树；启动器退出时会终止它拉起的所有实例

## 目录结构

```
dsh-launcher/
├── src/                # React 前端
│   ├── pages/          # Home / Instances / Download / InstanceDetail / SettingsPage
│   ├── components/     # 通用组件（Modal、图标、徽标…）
│   ├── store.tsx       # 全局状态（实例列表 / 运行状态 / Toast）
│   ├── api.ts          # Tauri invoke / event 封装
│   └── index.css       # PCL2 风格主题（深浅双主题）
├── src-tauri/
│   └── src/
│       ├── lib.rs      # 命令注册 + 退出清理
│       ├── settings.rs # 全局设置（便携式 dsh-launcher.json）
│       ├── instance.rs # 实例元数据 / CRUD / 目录布局
│       ├── versions.rs # npm registry 版本列表 + 按实例安装/卸载
│       ├── process.rs  # 进程拉起 / 停止 / 日志流 / 健康检查 / token URL 解析
│       └── state.rs    # 运行状态与日志环形缓冲
└── dsh-launcher.json   # 全局设置（首次运行后生成，与 exe 同目录）
```

## 开发

```bash
npm install
npm run tauri dev     # 开发模式（带热重载）
npm run tauri build   # 打包 Windows 安装包 / exe
```

Rust 侧单元测试（semver 通道分组 / token URL 解析 / env 校验等）：

```bash
cd src-tauri && cargo test
```

前置要求：Node.js ≥ 20、Rust stable（`x86_64-pc-windows-msvc`）、
VS Build Tools（C++ 工作负载）、WebView2（Win10/11 自带）。

## 安全设计

- 实例进程拉起为**纯参数列表调用**，不经任何 shell：参数只含 node 路径、
  实例目录内派生路径、数字端口与固定常量
- 安装版本时，版本号先经 semver 校验，再写入 `pkg/package.json` 的
  dependencies（不进命令行），随后执行参数固定的 `npm install`
- 实例自定义凭据不经过启动器进程，由 dsh Web UI 写入各实例独立的
  `home/.credentials.yaml`
- 删除实例、卸载 dsh 均有确认弹窗；启动器退出时自动清理子进程

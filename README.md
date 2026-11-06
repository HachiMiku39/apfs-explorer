# OpenAPFS for Windows — v0.1 read-only development

免费、开源的 Windows APFS 用户态工具，Fork 自 [APFS Explorer](https://github.com/enesilhaydin/apfs-explorer)，保留其版权与 GPL-3.0-only 许可证。Rust 解析器、WinFsp 盘符接口及 Tauri GUI 均复用上游。修改日期：2026-11-06。

**本 Fork v0.1 仅允许读取。任何 `--writable`、快照创建/删除/回滚、克隆写入和物理磁盘写句柄均被拒绝。** 上游的写入声明、历史测试及 CHANGELOG 不代表本 Fork 已通过验证。请先使用可丢弃镜像；CI 通过也不代表生产级可靠。

## 获取与运行

正式发布采用 [GitHub Actions](https://github.com/HachiMiku39/apfs-explorer/actions)；ARM64、x64、x86 三套作业分别产生便携 ZIP、MSI、对应源码 ZIP 和 SHA256SUMS，并检查 EXE/DLL 的 PE 架构。本次经用户明确授权，在 Parallels Windows 11 ARM64 内完成本地离线构建，不能记为云编译成功。当前仍是开发候选版，正式发布前需完成 [TESTING.md](TESTING.md) 中的其余验收。

先从 [WinFsp 官方](https://github.com/winfsp/winfsp/releases/tag/v2.1) 安装 WinFsp 2.1。OpenAPFS MSI 不静默下载或安装第三方驱动。GUI 使用 WinFsp Launcher；便携版请使用管理员终端运行 `apfs-mount.exe`，无需注册 Launcher。GUI 暂提供中英核心操作和语言选择，技术错误保留原文。

```powershell
# 使用备份镜像进行初次试验
.\apfs-mount.exe volume list --image C:\APFSTest\disk.img --json
.\apfs-mount.exe --image C:\APFSTest\disk.img --mount M:
# 或明确选择磁盘及卷（盘号必须来自本次设备发现）
.\apfs-mount.exe volume list --drive 2 --json
.\apfs-mount.exe --drive 2 --volume-index 0 --mount M:
```

在资源管理器打开 M:，将文件复制到 Windows 本地磁盘。挂载终端按 Enter 卸载；GUI 点“卸载”。停止复制、完成卸载后再断开设备；GUI 在挂载中或仍有已跟踪盘符时会阻止关闭，请先逐一卸载；卸载失败时保留跟踪状态并显示错误。不会将“卸载虚拟盘符”等同于物理设备弹出。

## 范围和限制

- 复用 GPT 64 位偏移、512/4096 扇区、APFS 4096 字节块、容器/卷选择、Unicode 名称和按范围读取。
- 未知不兼容格式、加密卷、Fusion、非连续 checkpoint 描述树拒绝挂载。镜像仅支持原始平面数据；压缩 UDIF DMG、sparsebundle 不在范围内。
- 4TiB 虚拟地址空间和跨 4GiB 文件偏移已通过单元测试；WOA 实盘读取/复制已验证 5GiB+257 字节稀疏文件、Unicode 和 384 字符相对路径。真实 4TB SSD 尚未验收；长路径仍取决于调用程序。
- 上游包含符号链接、硬链接读取和 xattr 解析；Windows 端不完整暴露 APFS 权限/xattr，不能据此制作保真备份。Windows 禁止的名称及大小写冲突需要进一步兼容验证。
- 三种架构已本地构建并加入云测试矩阵。ARM64 在 WOA 上完成真实 SSD 只读验收；x64/x86 仅完成 WOA 兼容环境中的构建和单元测试，Windows 10、Intel/AMD 实机及真实 32 位系统仍待验收。

## 本地验证记录（2026-11-06）

在 Windows 11 ARM64 / WinFsp 2.1 上，核心与挂载库 467 项测试通过，另有 1 项只读 CLI 回归通过。x64/x86 库测试也在 WOA 兼容环境通过；x86 WinFsp 原生长度溢出回归通过。

128GB TOSHIBA APFS SSD 上的 2,058 个自建文件，在 Windows 读取、复制后的 SHA-256 全部与 macOS 清单相同。打开写句柄与新建请求均返回介质写保护；正常卸载后，macOS 再次核对全部哈希和原生链接/xattr，`fsck_apfs -n` 检查通过。盘内原有用户文件未参与测试。测试通过不代表生产级可靠，也不代表 Windows 已完整暴露 APFS 元数据。

## 开发和分期

[研究与复用](docs/RESEARCH.md) · [架构](ARCHITECTURE.md) · [验证矩阵](TESTING.md) · [安全](SECURITY.md)

v0.1 完成只读保护和云构建后，先验证镜像在 Windows 的挂载/复制/卸载，再验收真实设备只读。后续写入阶段仅在可丢弃普通镜像上审查现有 `apfs-write`，覆盖创建/删除/移动/重命名、随机覆盖/追加、截断/扩展、分配回收、checkpoint/flush、故障恢复。真实磁盘写入必须另有明确授权；v0.1 没有解除限制的开关。

许可证见 [LICENSE](LICENSE)；依赖许可证由 `cargo deny --locked check licenses` 检查，保留第三方声明。禁止以关闭校验或强改元数据掩盖错误。

## Windows 架构

| 包名后缀 | 程序架构 | 目标 |
|---|---|---|
| `windows-arm64` | 原生 ARM64 | ARM64 Windows 10/11 |
| `windows-x64` | 64 位 Intel/AMD | x64 Windows 10/11 |
| `windows-x86` | 32 位 x86 | Windows 10 32 位，以及 64 位系统的 32 位兼容环境 |

Windows 11 没有 32 位 x86 操作系统版本；`windows-x86` 表示程序位数。
ARM 指 ARM64，不包含 ARM32。请选择与系统匹配的原生包，WinFsp 驱动仍需安装。
构建输出示例：`OpenAPFS-0.1.0-windows-arm64.zip` / `.msi`，其余架构同理。
32 位进程也使用 64 位文件偏移，超大文件按块读取；需要一次性装入内存的属性或
资源叉若超出进程可表示范围则明确报错。详见 [多架构说明](docs/WINDOWS_ARCHITECTURES.md)。

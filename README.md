BooBoo 是一款为 macOS 深度定制的生产力聚合组件，旨在用**一个极轻量级的原生 App** 解决你工作流中的高频零散需求，让你告别菜单栏挤满五个后台图标的烦恼。

## 🌟 核心功能 (5-in-1 瑞士军刀)

1. **OCR 截图与多端翻译**：支持全局热键框选与放大镜精准选区，框选即提取文字并送往云端翻译。支持自由切换引擎（百度、有道、火山、腾讯等），并拥有精致的实时悬浮窗结果呈现。
2. **超级右键助手 (新建文件)**：集成 Finder Sync 扩展，让 Mac 右键瞬间拥有“新建 Word/PPT/Excel、文本、Python 等文件”的能力，并支持读取您的自定义专属模板文件夹。
3. **窗口高亮与置顶贴图 (Pin Image)**：智能识别鼠标下方的目标窗口，单击即截取并转化为一块可自由拖拽的置顶“便利贴”，工作对照、代码比对的极佳利器。
4. **独立滚动反转 (Scroll Reverser)**：外接鼠标用户福音！从系统底层完美解耦外接鼠标与原生触控板的滑动方向，实现独立滚动逻辑控制。
5. **快速录屏与剪贴板监听**：集成极简的屏幕动态捕捉分发，并可在后台静默记录核心剪贴板动态。

> **这份 `README.md` 面向开发者与维护者**，主要记录：**打包分发指引、数据架构、模块边界、关键权限与故障排查**。

## 🚀 独立打包与公证 (面向 GitHub Release 分发)

本项目已脱离 Mac App Store 的诸多上架束缚，旨在通过 GitHub / 独立网站直接提供功能火力全开的二进制包。项目自带了一键 DMG 打包发布脚本：

1. **准备公证凭据**（仅需一次）：
   打开终端运行 `xcrun notarytool store-credentials booboo-notary --apple-id <你的AppleID> --team-id J8R8HSKWKD --password <App专用密码>`
2. **执行打包**：在项目根目录运行以下脚本：
   ```bash
   sh ./scripts/build-dmg.sh
   ```
3. 这个脚本将自动执行 **Release 编译 → 深度签名 → 制作 DMG → 提交 Apple 服务器公证 → Stapling 注入票据**。完成后，`build/` 目录下带有数字签名的 `.dmg` 即可直接上传至 GitHub Releases。

## 🛠️ 快速运行与调试

1. 用 Xcode 打开 `BooBoo.xcodeproj`，选择 Debug 模式直接 Run。
2. 若需调试高级功能，请前往 **系统设置 → 隐私与安全性** 中授予 **“屏幕录制”**（用于 OCR）以及 **“辅助功能/输入监控”**（用于贴图捕获其他应用边界及鼠标滚轮反转拦截）。

## 权限（重要）

截图与 OCR 通常需要 **“屏幕录制”** 权限，否则 `ScreenCaptureKit` 可能返回空白帧或失败。

- 系统设置 → 隐私与安全性 → 屏幕录制 → 允许 **BooBoo**
- 代码侧会做预检：`CGPreflightScreenCaptureAccess()`（见 `ScreenCaptureService`）

常见提示（来自 `TranslationViewModel`）：
> 需要开启“屏幕录制”权限：系统设置 -> 隐私与安全性 -> 屏幕录制，允许 BooBoo

**滚动反转**（可选，对标 Scroll Reverser）需在 **系统设置 → 隐私与安全性 → 输入监控** 中允许 BooBoo；工程已声明 `com.apple.security.device.input-monitoring` 与用途说明（Info）。触摸板与鼠标通过 `scrollWheelEventIsContinuous` 等字段做启发式区分，个别设备可能与预期不完全一致。

## 核心用户流程（事件/数据流）

**全局热键 → 选区截图 → OCR → 翻译（含缓存）→ UI 展示**

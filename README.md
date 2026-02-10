# 👻 BooBoo - 极速 OCR 翻译与屏幕钉图工具

<p align="center">
  <img src="src-tauri/icons/128x128.png" width="128" alt="BooBoo Logo">
  <br>
  <b>轻量 · 极速 · 优雅 的桌面翻译助手</b>
</p>

---

## 🌟 核心特性

- **🚀 零延迟截图**: 采用 BMP 无损传输与异步预加载技术，实现“即点即扫”的丝滑体验。
- **🔍 强大 OCR 引擎**: 
  - 支持 **PaddleOCR** (本地离线，高精度)。
  - 支持 **Windows Native OCR** (系统内置，响应极快)。
- **🌐 智能翻译**: 内置多语种翻译能力，支持结果框高度自适应。
- **✨ 极简视觉**: 
  - 现代圆角矩形 (Squircle) 设计。
  - 针对 Windows 优化的系统默认字体。
  - 极简滚动条与沉浸式交互细节。
- **⚙️ 全局快捷键**: 自定义截图、钉图、翻译等全局触发，支持托盘常驻。
- **🛠️ 开发者友好**: 基于 **Tauri 2.0 (Rust + Svelte)** 开发，内存占用仅为 Electron 的 1/3。

## 📥 安装与运行

### 直接下载
请前往 [Releases](https://github.com/anchorwon/booboo/releases) 页面下载最新的安装包：
- **BooBoo-setup.exe**: 推荐安装版。
- **booboo.exe**: 即开即用的单体版。

### 本地编译
1. **环境准备**:
   - 安装 [Node.js](https://nodejs.org/) (建议 v18+)。
   - 安装 [Rust 环境](https://rustup.rs/)。
2. **克隆项目**:
   ```bash
   git clone https://github.com/anchorwon/booboo.git
   cd booboo
   ```
3. **安装依赖**:
   ```bash
   npm install
   ```
4. **运行开发版**:
   ```bash
   npm run tauri dev
   ```
5. **构建正式版**:
   ```bash
   npm run tauri build
   ```

## 🛠️ 技术栈

- **Frontend**: [Svelte 5](https://svelte.dev/) + [Vite](https://vitejs.dev/)
- **Backend**: [Rust](https://www.rust-lang.org/) + [Tauri 2.0](https://tauri.app/)
- **Styling**: Native CSS (Modern & Clean UI)
- **OCR**: PaddleOCR-json & Windows Runtime API

## 📝 路线图 (Roadmap)

- [x] 全面品牌更名为 BooBoo
- [x] 优化截图加载路径，解决白屏闪烁
- [x] 引入 200ms 窗口焦点缓冲机制
- [x] 识别/翻译框高度自适应及最大高度限制
- [ ] 增加更多在线翻译引擎支持
- [ ] 支持屏幕取色与尺子工具插件

## 🤝 贡献与反馈

欢迎提交 Issue 或 Pull Request 来帮助 BooBoo 变得更好！

---

**BooBoo** - *让你的文字识别与翻译体验从此轻盈如灵。*

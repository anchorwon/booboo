<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";

  interface AppConfig {
    ocr_engine: string;
    translate_engine: string;
    youdao_app_key: string;
    youdao_app_secret: string;
    coze_api_key: string;
  }

  let config: AppConfig = $state({
    ocr_engine: "paddle",
    translate_engine: "google",
    youdao_app_key: "",
    youdao_app_secret: "",
    coze_api_key: ""
  });

  let message = $state("");
  let verifyMessage = $state("");
  let isVerifying = $state(false);
  let activeTab = $state("general"); // general | ocr | translate | shortcut | about
  let { onClose } = $props();

  const tabs = [
    { id: "general", label: "通用", icon: "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.35 19.43,11.03L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.97 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.97 4.46,5.05 4.34,5.27L2.34,8.73C2.22,8.95 2.27,9.22 2.46,9.37L4.57,11.03C4.53,11.35 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.22,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.03 4.95,18.95L7.44,17.95C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.68 16.04,18.34 16.56,17.95L19.05,18.95C19.27,19.03 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z" },
    { id: "ocr", label: "OCR 设置", icon: "M16,17V14H9V10H16V7L21,12L16,17M14,2H4A2,2 0 0,0 2,4V20A2,2 0 0,0 4,22H14A2,2 0 0,0 16,20V19H14V20H4V4H14V5H16V4A2,2 0 0,0 14,2Z" },
    { id: "translate", label: "翻译服务", icon: "M12.87,15.07L10.33,12.56L10.36,12.53C12.1,10.59 13.34,8.36 14.07,6H17V4H10V2H8V4H1V6H12.17C11.5,7.92 10.44,9.75 9,11.35C8.07,10.32 7.3,9.19 6.69,8H4.69C5.42,9.63 6.42,11.17 7.67,12.56L2.58,17.58L4,19L9,14L12.11,17.11L12.87,15.07M18.5,10H16.5L12,22H14L15.12,19H19.87L21,22H23L18.5,10M15.88,17L17.5,12.65L19.12,17H15.88Z" },
    { id: "shortcut", label: "快捷键", icon: "M21,16.5C21,16.88 20.79,17.21 20.47,17.38L12.57,21.82C12.41,21.94 12.21,22 12,22C11.79,22 11.59,21.94 11.43,21.82L3.53,17.38C3.21,17.21 3,16.88 3,16.5V7.5C3,7.12 3.21,6.79 3.53,6.62L11.43,2.18C11.59,2.06 11.79,2 12,2C12.21,2 12.41,2.06 12.57,2.18L20.47,6.62C20.79,6.79 21,7.12 21,7.5V16.5Z" },
    { id: "about", label: "关于", icon: "M11,9H13V7H11V9M12,20C7.59,20 4,16.41 4,12C4,7.59 7.59,4 12,4C16.41,4 20,7.59 20,12C20,16.41 16.41,20 12,20M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M11,17H13V11H11V17Z" },
  ];

  onMount(async () => {
    try {
      config = await invoke<AppConfig>("get_config");
    } catch (e) {
      console.error("Failed to load config:", e);
      message = "加载配置失败";
    }
  });

  async function verifyYoudao() {
    if (!config.youdao_app_key || !config.youdao_app_secret) {
      verifyMessage = "请先填写应用 ID 和密钥";
      return;
    }
    
    isVerifying = true;
    verifyMessage = "正在验证...";
    try {
      const result = await invoke<string>("verify_youdao_id_and_key", { 
        appKey: config.youdao_app_key, 
        appSecret: config.youdao_app_secret 
      });
      verifyMessage = result;
    } catch (e) {
      verifyMessage = String(e);
    } finally {
      isVerifying = false;
    }
  }

  async function saveSettings() {
    try {
      await invoke("save_config", { newConfig: config });
      message = "设置已保存";
      setTimeout(() => {
        message = "";
      }, 2000);
    } catch (e) {
      console.error("Failed to save config:", e);
      message = `保存失败: ${e}`;
    }
  }
</script>

<div class="dashboard-overlay" transition:fade={{ duration: 200 }}>
  <div class="dashboard-container" transition:fly={{ y: 20, duration: 300 }}>
    <!-- Sidebar -->
    <div class="sidebar">
      <div class="sidebar-header">
        <div class="logo">BooBoo</div>
      </div>
      <nav class="nav-menu">
        {#each tabs as tab}
          <button 
            class="nav-item" 
            class:active={activeTab === tab.id} 
            onclick={() => activeTab = tab.id}
          >
            <svg viewBox="0 0 24 24" width="18" height="18">
              <path fill="currentColor" d={tab.icon} />
            </svg>
            <span>{tab.label}</span>
          </button>
        {/each}
      </nav>
      <div class="sidebar-footer">
        <button class="close-btn" onclick={onClose}>
          退出设置
        </button>
      </div>
    </div>

    <!-- Main Content -->
    <div class="main-content">
      <header class="content-header">
        <h1>{tabs.find(t => t.id === activeTab)?.label}</h1>
        <button class="save-top-btn" onclick={saveSettings}>保存更改</button>
      </header>

      <div class="content-body">
        {#if activeTab === 'general'}
          <section class="settings-section" in:fade>
            <div class="section-card">
              <div class="setting-item">
                <div class="setting-info">
                  <span class="label">开机自启</span>
                  <span class="desc">系统启动时自动运行 BooBoo</span>
                </div>
                <label class="switch">
                  <input type="checkbox" checked disabled>
                  <span class="slider"></span>
                </label>
              </div>
              <div class="setting-divider"></div>
              <div class="setting-item">
                <div class="setting-info">
                  <span class="label">深色模式</span>
                  <span class="desc">跟随系统外观设置</span>
                </div>
                <label class="switch">
                  <input type="checkbox" checked>
                  <span class="slider"></span>
                </label>
              </div>
            </div>
          </section>
        {/if}

        {#if activeTab === 'ocr'}
          <section class="settings-section" in:fade>
            <div class="section-card">
              <div class="setting-item">
                <div class="setting-info">
                  <span class="label">OCR 引擎</span>
                  <span class="desc">选择用于文本识别的服务</span>
                </div>
                <select bind:value={config.ocr_engine}>
                  <option value="paddle">PaddleOCR (本地高性能)</option>
                  <option value="windows">Windows Native (系统内置)</option>
                </select>
              </div>
            </div>
          </section>
        {/if}

        {#if activeTab === 'translate'}
          <section class="settings-section" in:fade>
            <div class="section-card">
              <div class="setting-item">
                <div class="setting-info">
                  <span class="label">翻译引擎</span>
                  <span class="desc">默认翻译服务提供商</span>
                </div>
                <select bind:value={config.translate_engine}>
                  <option value="google">Google Translate (免 Key)</option>
                  <option value="youdao">有道翻译 (推荐, 需 Key)</option>
                </select>
              </div>
              
              {#if config.translate_engine === 'youdao'}
                <div class="config-sub-card">
                  <div class="input-group">
                    <label>应用 ID (App Key)</label>
                    <input type="text" bind:value={config.youdao_app_key} placeholder="请输入有道应用ID" />
                  </div>
                  <div class="input-group">
                    <label>应用密钥 (App Secret)</label>
                    <input type="password" bind:value={config.youdao_app_secret} placeholder="请输入密钥" />
                  </div>
                  <div class="verify-action">
                    <button class="action-btn" onclick={verifyYoudao} disabled={isVerifying}>
                      {isVerifying ? "正在请求..." : "验证配置"}
                    </button>
                    {#if verifyMessage}
                      <span class="status-tip" class:error={verifyMessage.includes('失败')}>{verifyMessage}</span>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          </section>
        {/if}

        {#if activeTab === 'shortcut'}
          <section class="settings-section" in:fade>
            <div class="section-card">
                <div class="setting-item">
                <div class="setting-info">
                    <span class="label">屏幕识图 (OCR)</span>
                    <span class="desc">触发截图并进行文字识别</span>
                </div>
                <div class="key-binder">Alt + Shift + A</div>
                </div>
                <div class="setting-divider"></div>
                <div class="setting-item">
                <div class="setting-info">
                    <span class="label">屏幕翻译</span>
                    <span class="desc">识别区域后自动翻译</span>
                </div>
                <div class="key-binder empty">点击设置</div>
                </div>
            </div>
            <p class="section-tip">当前版本的快捷键仅供预览，暂不支持在线修改。</p>
          </section>
        {/if}

        {#if activeTab === 'about'}
          <section class="settings-section" in:fade>
            <div class="about-card">
              <div class="app-logo">B</div>
              <h3>BooBoo OCR</h3>
              <p>v0.1.0-alpha</p>
              <div class="links">
                <a href="#">官方网站</a>
                <span class="dot">·</span>
                <a href="#">开源协议</a>
                <span class="dot">·</span>
                <a href="#">反馈建议</a>
              </div>
            </div>
          </section>
        {/if}
        
        {#if message}
          <div class="toast-message" in:fade out:fade>{message}</div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .dashboard-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dashboard-container {
    width: 800px;
    height: 600px;
    background: #fdfdfd;
    border-radius: 16px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    overflow: hidden;
    display: flex;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  /* Sidebar */
  .sidebar {
    width: 220px;
    background: #f1f2f4;
    border-right: 1px solid #e2e4e7;
    display: flex;
    flex-direction: column;
    padding: 24px 0;
  }

  .sidebar-header {
    padding: 0 24px 32px;
  }

  .logo {
    font-size: 24px;
    font-weight: 800;
    color: #4f46e5;
    letter-spacing: -1px;
  }

  .nav-menu {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0 12px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: #4b5563;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s;
    text-align: left;
  }

  .nav-item:hover {
    background: #e5e7eb;
    color: #111827;
  }

  .nav-item.active {
    background: #e0e7ff;
    color: #4f46e5;
  }

  .sidebar-footer {
    padding: 0 24px;
  }

  .close-btn {
    width: 100%;
    padding: 10px;
    background: transparent;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 13px;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: #fff;
    color: #e11d48;
    border-color: #fca5a5;
  }

  /* Main Content */
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #fff;
  }

  .content-header {
    padding: 24px 32px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid #f3f4f6;
  }

  .content-header h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    color: #111827;
  }

  .save-top-btn {
    padding: 8px 16px;
    background: #4f46e5;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 2px 4px rgba(79, 70, 229, 0.2);
  }

  .save-top-btn:hover {
    background: #4338ca;
  }

  .content-body {
    flex: 1;
    padding: 32px;
    overflow-y: auto;
    position: relative;
  }

  .settings-section {
    max-width: 600px;
  }

  .section-card {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 24px;
  }

  .setting-item {
    padding: 18px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 15px;
    font-weight: 600;
    color: #1f2937;
  }

  .desc {
    font-size: 12px;
    color: #6b7280;
  }

  .setting-divider {
    height: 1px;
    background: #f3f4f6;
    margin: 0 20px;
  }

  /* Controls */
  select {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid #d1d5db;
    font-size: 13px;
    background: #f9fafb;
    outline: none;
    min-width: 160px;
  }

  .config-sub-card {
    background: #f9fafb;
    padding: 20px;
    border-top: 1px solid #f3f4f6;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-group label {
    font-size: 12px;
    font-weight: 500;
    color: #6b7280;
  }

  .input-group input {
    padding: 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 14px;
    background: #fff;
    outline: none;
  }

  .verify-action {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .action-btn {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid #4f46e5;
    color: #4f46e5;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .action-btn:hover:not(:disabled) {
    background: #4f46e5;
    color: white;
  }

  .status-tip {
    font-size: 12px;
    color: #10b981;
  }

  .status-tip.error {
    color: #ef4444;
  }

  /* Switch */
  .switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #d1d5db;
    transition: .3s;
    border-radius: 34px;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: .3s;
    border-radius: 50%;
  }

  input:checked + .slider {
    background-color: #4f46e5;
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  /* Key Binder */
  .key-binder {
    padding: 6px 12px;
    background: #f3f4f6;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #4b5563;
  }

  .key-binder.empty {
    color: #9ca3af;
    font-style: italic;
    font-weight: 400;
    cursor: pointer;
  }

  .section-tip {
    font-size: 12px;
    color: #9ca3af;
    margin-top: -12px;
    margin-left: 4px;
  }

  /* About */
  .about-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px 0;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
  }

  .app-logo {
    width: 64px;
    height: 64px;
    background: #4f46e5;
    color: white;
    font-size: 32px;
    font-weight: 900;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 16px;
    margin-bottom: 20px;
    box-shadow: 0 8px 16px rgba(79, 70, 229, 0.2);
  }

  .about-card h3 {
    margin: 0 0 8px;
    font-size: 18px;
  }

  .about-card p {
    color: #6b7280;
    margin: 0 0 24px;
  }

  .links {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .links a {
    color: #4f46e5;
    text-decoration: none;
    font-size: 14px;
  }

  .dot {
    color: #d1d5db;
  }

  /* Toast */
  .toast-message {
    position: absolute;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: #1f2937;
    color: white;
    padding: 8px 16px;
    border-radius: 99px;
    font-size: 13px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }
</style>


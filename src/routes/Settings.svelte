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
    shortcut: string;
    shortcut_pin: string;
  }

  let isInitialLoad = true;
  let saveTimeout: any;

  let engines = [
    { id: "google", name: "Google 翻译", logo: "/google-logo.webp" },
    { id: "youdao", name: "有道翻译", logo: "/youdao-logo.png" },
    { id: "tencent", name: "腾讯翻译君", logo: "/tencent-fanyi-logo.png" }
  ];
  let config: AppConfig = $state({
    ocr_engine: "paddle",
    translate_engine: "google",
    youdao_app_key: "",
    youdao_app_secret: "",
    coze_api_key: "",
    shortcut: "Alt+Shift+A"
  });

  let message = $state("");
  let verifyMessage = $state("");
  let isVerifying = $state(false);
  let activeTab = $state("general"); // general | ocr | translate | shortcut | about
  let { onClose } = $props();
  
  let recordingMode = $state<string | null>(null); // 'ocr' | 'pin' | null

  const tabs = [
    { id: "general", label: "通用", icon: "/settings.svg" },
    { id: "ocr", label: "OCR 设置", icon: "/ocr.svg" },
    { id: "translate", label: "翻译服务", icon: "/fanyi.svg" },
    { id: "shortcut", label: "快捷键", icon: "/keyboard.svg" },
    { id: "about", label: "关于", icon: "/alert-square.svg" },
  ];

  onMount(async () => {
    try {
      const loadedConfig = await invoke<AppConfig>("get_config");
      // Update config values without triggering auto-save if possible, 
      // but in Svelte 5 $effect will run anyway. 
      // We'll use isInitialLoad to prevent saving immediately after load.
      Object.assign(config, loadedConfig);
      if (!config.shortcut) config.shortcut = "Alt+Shift+A";
      if (!config.shortcut_pin) config.shortcut_pin = "Alt+Shift+S";
      
      await tick();
      isInitialLoad = false;
    } catch (e) {
      console.error("Failed to load config:", e);
      message = "加载配置失败";
    }
  });

  // Auto-save logic
  $effect(() => {
    // Access properties to track them
    const currentConfig = {
      ocr_engine: config.ocr_engine,
      translate_engine: config.translate_engine,
      youdao_app_key: config.youdao_app_key,
      youdao_app_secret: config.youdao_app_secret,
      coze_api_key: config.coze_api_key,
      shortcut: config.shortcut,
      shortcut_pin: config.shortcut_pin
    };

    if (isInitialLoad) return;

    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      try {
        await invoke("save_config", { newConfig: currentConfig });
        console.log("Settings auto-saved");
      } catch (e) {
        console.error("Auto-save failed:", e);
      }
    }, 500); // 500ms debounce
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
      if (recordingMode) {
         recordingMode = null; // Cancel recording if saving
      }
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
  
  function handleKeyDown(e: KeyboardEvent) {
    if (!recordingMode) return;
    e.preventDefault();
    e.stopPropagation();
    
    // Ignore standalone modifier presses
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    
    const modifiers: string[] = [];
    if (e.ctrlKey) modifiers.push("Ctrl"); // Tauri uses "Control" or "Ctrl"? "Ctrl" works usually or "CommandOrControl" 
    // Actually tauri-plugin-global-shortcut expects specific names.
    // "Alt", "Control", "Shift", "Super" (Windows key)
    
    if (e.ctrlKey) modifiers.push("Ctrl");
    if (e.altKey) modifiers.push("Alt");
    if (e.shiftKey) modifiers.push("Shift");
    if (e.metaKey) modifiers.push("Super"); 
    
    let key = e.key.toUpperCase();
    if (key === " ") key = "Space";
    
    // Combine
    if (modifiers.length > 0) {
        const shortcutStr = [...modifiers, key].join("+");
        if (recordingMode === 'ocr') config.shortcut = shortcutStr;
        if (recordingMode === 'pin') config.shortcut_pin = shortcutStr;
        recordingMode = null;
    } else {
        // Allow single keys like F1-F12?
        if (key.startsWith("F")) {
            if (recordingMode === 'ocr') config.shortcut = key;
            if (recordingMode === 'pin') config.shortcut_pin = key;
            recordingMode = null;
        }
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="dashboard-overlay" transition:fade={{ duration: 200 }}>
  <div class="dashboard-container" transition:fly={{ y: 20, duration: 300 }} data-tauri-drag-region>
    <!-- Sidebar -->
    <div class="sidebar" data-tauri-drag-region>
      <div class="sidebar-header" data-tauri-drag-region>
        <div class="logo">
          <img src="/logo.png" alt="BooBoo" />
          <span>BooBoo</span>
        </div>
      </div>
      <nav class="nav-menu">
        {#each tabs as tab}
          <button 
            class="nav-item" 
            class:active={activeTab === tab.id} 
            onclick={() => activeTab = tab.id}
          >
            {#if tab.icon.includes('/') || tab.icon.endsWith('.svg')}
              <img src={tab.icon} alt={tab.label} style="width: 18px; height: 18px; color: inherit;" />
            {:else}
              <svg viewBox="0 0 24 24" width="18" height="18">
                <path fill="currentColor" d={tab.icon} />
              </svg>
            {/if}
            <span>{tab.label}</span>
          </button>
        {/each}
      </nav>
      <div class="sidebar-footer">
        <!-- Sidebar footer button removed as requested -->
      </div>
    </div>

    <!-- Main Content -->
    <div class="main-content">
      <header class="content-header" data-tauri-drag-region>
        <h1>{tabs.find(t => t.id === activeTab)?.label}</h1>
        <button class="icon-close-btn" onclick={onClose} title="关闭仪表盘">
          <svg viewBox="0 0 24 24" width="20" height="20">
            <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z" />
          </svg>
        </button>
      </header>

      <div class="content-body">
        {#if activeTab === 'general'}
          <section class="settings-section" in:fade>
            <div class="section-group">
              <h2>界面模式</h2>
              <div class="section-card">
                 <!-- Placeholder for the mode icons like in screenshot -->
                 <div class="setting-item" style="justify-content: center; gap: 40px; padding: 32px;">
                    <div style="text-align: center;">
                       <div style="width: 100px; height: 60px; background: #eee; border-radius: 8px; border: 2px solid #0099ff; margin-bottom: 8px;"></div>
                       <div class="label" style="font-size: 13px;">效率模式</div>
                    </div>
                    <div style="text-align: center; opacity: 0.6;">
                       <div style="width: 100px; height: 60px; background: #eee; border-radius: 8px; margin-bottom: 8px;"></div>
                       <div class="label" style="font-size: 13px;">经典模式</div>
                    </div>
                 </div>
              </div>
            </div>

            <div class="section-group">
              <h2>高级设置</h2>
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
            <div class="section-group">
              <div class="section-card">
                <div class="engine-list">
                  {#each engines as engine}
                    <label class="engine-option" class:selected={config.translate_engine === engine.id}>
                      <div class="engine-main">
                        <input type="radio" value={engine.id} bind:group={config.translate_engine} />
                        {#if engine.logo}
                          <img src={engine.logo} alt={engine.name} class="engine-logo" />
                        {/if}
                        <span class="engine-name">{engine.name}</span>
                      </div>
                      {#if config.translate_engine === engine.id}
                        <span class="check-badge">
                          <svg viewBox="0 0 24 24" width="18" height="18">
                            <path fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                          </svg>
                        </span>
                      {/if}
                    </label>
                  {/each}
                </div>

                {#if config.translate_engine === 'youdao'}
                  <div class="youdao-config">
                    <div class="config-title">有道翻译配置</div>
                    <div class="config-form">
                      <div class="input-row">
                        <label>应用 ID</label>
                        <input type="text" bind:value={config.youdao_app_key} placeholder="App Key" />
                      </div>
                      <div class="input-row">
                        <label>应用密钥</label>
                        <input type="password" bind:value={config.youdao_app_secret} placeholder="App Secret" />
                      </div>
                      <div class="form-footer">
                        <button class="verify-btn" onclick={verifyYoudao} disabled={isVerifying}>
                          {isVerifying ? "验证中..." : "验证配置"}
                        </button>
                        {#if verifyMessage}
                          <span class="status-tip" class:error={verifyMessage.includes('失败')}>{verifyMessage}</span>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          </section>
        {/if}

        {#if activeTab === 'shortcut'}
          <section class="settings-section" in:fade>
            <div class="section-card">
                <div class="setting-item">
                <div class="setting-info">
                    <span class="label">屏幕识图 (OCR)</span>
                    <span class="desc">全局截图快捷键</span>
                </div>
                <!-- Interactive Key Binder -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div 
                   class="key-binder" 
                   class:recording={recordingMode === 'ocr'}
                   onclick={() => recordingMode = 'ocr'}
                >
                   {recordingMode === 'ocr' ? "按下快捷键..." : config.shortcut}
                </div>
                </div>
                <div class="setting-divider"></div>
                <div class="setting-item">
                <div class="setting-info">
                    <span class="label">屏幕贴图 (Pin)</span>
                    <span class="desc">将截图固定的屏幕上</span>
                </div>
                <div 
                   class="key-binder" 
                   class:recording={recordingMode === 'pin'}
                   onclick={() => recordingMode = 'pin'}
                >
                   {recordingMode === 'pin' ? "按下快捷键..." : config.shortcut_pin}
                </div>
                </div>
            </div>
            <!-- Tip removed as requested -->
          </section>
        {/if}

        {#if activeTab === 'about'}
          <section class="settings-section" in:fade>
            <div class="about-card">
              <img src="/logo.png" alt="BooBoo" class="about-logo" />
              <h3>BooBoo OCR</h3>
              <p>v0.1.0-alpha</p>
              <div class="links">
                <a href="https://github.com/anchorwon/booboo" target="_blank" rel="noreferrer">GitHub 项目主页</a>
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
  :global(html, body) {
    background: transparent !important;
  }

  .dashboard-overlay {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
  }

  .dashboard-container {
    width: 100%;
    height: 100%;
    background: transparent;
    border-radius: 0;
    overflow: hidden;
    display: flex;
    font-family: system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, "Microsoft YaHei", sans-serif;
  }

  /* Sidebar: Milky Glass effect for maximum occlusion and premium feel */
  /* Sidebar: Milky Glass effect for maximum occlusion and premium feel */
  .sidebar {
    position: relative;
    width: 240px;
    /* Restored the clean-cut border style as per revert request */
    background: rgba(255, 255, 255, 0.9); 
    border-right: 1px solid rgba(0, 0, 0, 0.04);
    display: flex;
    flex-direction: column;
    padding: 32px 0;
    z-index: 10;
    transform: translateZ(0);
    overflow: hidden;
  }

  /* Layer 1: The "Milky" wash - Pure and Grainless */
  .sidebar::before {
    content: "";
    position: absolute;
    inset: 0;
    /* Increased white-wash and brightness to combat the "dark" feeling */
    background-color: rgba(255, 255, 255, 0.6);
    /* Brightness increased to 1.4 to pop the colors and clean the gray muddy feel */
    backdrop-filter: blur(80px) saturate(4) contrast(0.2) brightness(1.4); 
    -webkit-backdrop-filter: blur(80px) saturate(4) contrast(0.2) brightness(1.4);
    z-index: -2;
    pointer-events: none;
  }

  /* Layer 2: Deep light diffusion layer */
  .sidebar::after {
    content: "";
    position: absolute;
    inset: 0;
    backdrop-filter: blur(300px);
    -webkit-backdrop-filter: blur(300px);
    z-index: -3;
    pointer-events: none;
  }

  .sidebar-header {
    padding: 0 28px 24px;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 22px;
    font-weight: 600;
    color: #000000; /* Pure black Logo */
    letter-spacing: -0.5px;
  }

  .logo img {
    width: 36px; 
    height: 36px;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(0,0,0,0.1); /* Subtle shadow for depth */
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
    padding: 10px 16px;
    border: none;
    background: transparent;
    color: #000000; /* Unified pure black */
    border-radius: 10px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 400; 
    transition: all 0.2s ease;
    text-align: left;
  }

  .nav-item:hover {
    background: rgba(0, 0, 0, 0.04);
  }

  .nav-item.active {
    background: rgba(0, 0, 0, 0.08); 
    color: #000000; /* Pure black for active */
    font-weight: 400; /* No bolding on active state */
  }

  .nav-item svg {
    color: inherit;
    transition: inherit;
  }

  /* Simplified pure black filter for all navigation icons */
  .nav-item img {
    filter: brightness(0); 
    transition: filter 0.2s ease;
  }

  /* Main Content: Light gray background for contrast (Restored) */
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #f8f9fb;
  }

  .content-header {
    padding: 28px 40px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(0, 0, 0, 0.04); /* Divider for the main title */
    margin-bottom: 8px;
  }

  .content-header h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 600; /* Reduced from 800 for a lighter feel */
    color: #000;
    letter-spacing: -0.5px;
  }

  .icon-close-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #666;
    cursor: pointer;
    transition: background 0.2s;
  }

  .icon-close-btn:hover {
    background: rgba(0, 0, 0, 0.05);
    color: #000;
  }

  .content-body {
    flex: 1;
    padding: 12px 40px 40px;
    overflow-y: auto;
  }

  .settings-section {
    max-width: 800px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .settings-section h2 {
    font-size: 16px;
    font-weight: 600; /* Reduced from 700 */
    color: #111827;
    margin: 8px 0 12px; /* Increased bottom margin for better spacing */
    letter-spacing: -0.2px;
  }

  /* Pure white cards with high rounding and ultra-light borders */
  .section-card {
    background: #ffffff;
    border: 1px solid rgba(0, 0, 0, 0.03);
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
    overflow: hidden;
  }

  .setting-item {
    padding: 20px 24px;
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
    color: #111827;
  }

  .desc {
    font-size: 13px;
    color: #8b949e;
  }

  .setting-divider {
    height: 1px;
    background: rgba(0, 0, 0, 0.03);
    margin: 0 24px;
  }

  /* Switch: QQ Blue */
  .switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
  }

  .switch input { opacity: 0; width: 0; height: 0; }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: #e5e7eb;
    transition: .3s;
    border-radius: 24px;
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
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  input:checked + .slider {
    background-color: #000; /* Black for sliders too */
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  /* Engine List & Options */
  .engine-list {
    display: flex;
    flex-direction: column;
    padding: 8px 0;
  }

  .engine-option {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 24px;
    cursor: pointer;
    transition: background 0.2s;
    border-bottom: 1px solid rgba(0, 0, 0, 0.02);
  }

  .engine-option:last-child {
    border-bottom: none;
  }

  .engine-option:hover {
    background: rgba(0, 0, 0, 0.01);
  }

  .engine-option.selected {
    background: transparent;
  }

  .engine-option input[type="radio"] {
    accent-color: #000; /* Black radio button */
  }

  .engine-main {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .engine-logo {
    width: 28px;
    height: 28px;
    object-fit: contain;
    border-radius: 6px;
  }

  .engine-name {
    font-size: 15px;
    font-weight: 500;
    color: #111827;
  }

  .check-badge {
    color: #000;
    font-weight: 800;
    font-size: 16px;
  }

  /* Youdao Config Sub-card */
  .youdao-config {
    background: #fbfbfc;
    border-top: 1px solid rgba(0, 0, 0, 0.04);
    padding: 24px 24px;
  }

  .config-title {
    font-size: 13px;
    font-weight: 700;
    color: #666;
    margin-bottom: 20px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .config-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 400px;
  }

  .input-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-row label {
    font-size: 13px;
    font-weight: 600;
    color: #374151;
  }

  .input-row input {
    padding: 10px 14px;
    border: 1px solid rgba(0, 0, 0, 0.1);
    border-radius: 8px;
    font-size: 14px;
    background: #fff;
    outline: none;
    transition: border-color 0.2s;
  }

  .input-row input:focus {
    border-color: #000;
  }

  .form-footer {
    padding-top: 8px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .verify-btn {
    padding: 8px 16px;
    background: #fff;
    border: 1px solid rgba(0, 0, 0, 0.12);
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    color: #111827;
    cursor: pointer;
    transition: all 0.2s;
  }

  .verify-btn:hover:not(:disabled) {
    background: #f3f4f6;
    border-color: rgba(0, 0, 0, 0.2);
  }

  .status-tip {
    font-size: 13px;
    color: #059669;
  }

  .status-tip.error {
    color: #dc2626;
  }

  /* Select & Input */
  select {
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid rgba(0, 0, 0, 0.1);
    font-size: 14px;
    background: #fafafa;
    outline: none;
    min-width: 160px;
  }

  .input-group input {
    padding: 10px 14px;
    border: 1px solid rgba(0, 0, 0, 0.08);
    border-radius: 8px;
    font-size: 14px;
    background: #fff;
    width: 100%;
    outline: none;
  }

  .input-group input:focus {
    border-color: #000;
  }

  .key-binder {
    padding: 8px 16px;
    background: #f0f2f5;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 600;
    color: #111827;
    cursor: pointer;
  }

  .key-binder.recording {
    background: #f8f9fb;
    color: #000;
    border: 1px solid #000;
  }

  .toast-message {
    position: absolute;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: #222;
    color: white;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    z-index: 1000;
  }

  /* About Card */
  .about-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    text-align: center;
    background: #fff;
    border-radius: 12px;
    border: 1px solid rgba(0, 0, 0, 0.03);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
  }

  .about-logo {
    width: 64px;
    height: 64px;
    margin-bottom: 16px;
    border-radius: 14px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  }

  .about-card h3 {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 700;
  }

  .about-card p {
    margin: 0 0 24px 0;
    color: #666;
    font-size: 13px;
  }

  .links {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .links a {
    color: #0066cc;
    text-decoration: none;
  }
  
  .links a:hover {
    text-decoration: underline;
  }

  .dot {
    color: #ccc;
  }
</style>


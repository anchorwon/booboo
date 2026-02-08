<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import CaptureOverlay from "$lib/CaptureOverlay.svelte";
  import Settings from "./Settings.svelte";

  let ocrResult = $state<string | null>(null);
  let translatedText = $state<string | null>(null);
  let isProcessing = $state(false);
  let isTranslating = $state(false);
  let isCapturing = $state(false);
  let hasAttempted = $state(false);
  let showSettings = $state(false);
  let statusMessage = $state("准备就绪");
  let targetLang = $state("zh-CN"); // Default to Chinese
  let detectedLang = $state("中文简体");
  const appWindow = getCurrentWindow();

  let isPinned = $state(false);

  const languages = [
    { code: "zh-CN", name: "中文简体" },
    { code: "en", name: "英文" }
  ];

  onMount(() => {
    const unlistenPromise = listen("shortcut-capture", () => {
      console.log("Frontend received shortcut-capture event!");
      startCapture();
    });

    const settingsUnlistenPromise = listen("open-settings", () => {
      console.log("Frontend received open-settings event!");
      showSettings = true;
      // Note: Backend already resized the window in this case, but no harm calling here too
      invoke("resize_dashboard_window", { mode: "dashboard" });
    });

    return () => {
      unlistenPromise.then(unlisten => unlisten());
      settingsUnlistenPromise.then(unlisten => unlisten());
    };
  });

  async function toggleSettings(show: boolean) {
    showSettings = show;
    await invoke("resize_dashboard_window", { mode: show ? "dashboard" : "normal" });
  }

  let captureBg = $state<string | null>(null);

  async function startCapture() {
    console.log("startCapture called, isProcessing:", isProcessing);
    if (isProcessing) return;
    try {
      // Step 0: Ensure window is hidden before capture to prevent flash
      // Since backend no longer calls show(), this is usually already hidden 
      // unless the user had the result window open.
      await appWindow.hide();
      
      isCapturing = true; 
      document.body.style.backgroundColor = "transparent";
      ocrResult = null;
      translatedText = null;
      
      console.log("Starting capture process...");
      
      // Step 1: Request full screen capture from backend
      statusMessage = "正在截取屏幕...";
      const filePath = await invoke<string>("capture_full_screen");
      console.log("Capture file path:", filePath);
      
      // Step 2: Convert to asset URL for display
      captureBg = convertFileSrc(filePath);
      
      // Step 3: SUCCESS! Now show the window as a fullscreen overlay
      await appWindow.setFullscreen(true);
      await appWindow.show();
      await appWindow.setFocus();
      
      statusMessage = "请选择区域";
    } catch (e) {
      console.error("Failed to start capture:", e);
      statusMessage = "截图失败";
      isCapturing = false;
      await appWindow.show();
    }
  }

  async function handleAreaSelect(rect) {
    const { x, y, width, height } = rect;
    isCapturing = false;
    document.body.style.backgroundColor = "";
    await appWindow.setFullscreen(false);

    isProcessing = true;
    hasAttempted = true;
    ocrResult = null;
    translatedText = null;
    
    try {
      // Calculate absolute physical coordinates for the backend crop
      const scaleFactor = await appWindow.scaleFactor();
      
      const result = await invoke<string>("ocr_capture_area", { 
        x: Math.round(x), 
        y: Math.round(y), 
        width: Math.round(width), 
        height: Math.round(height) 
      });
      ocrResult = result;
      
      if (result && result.trim()) {
        detectedLang = /[\u4e00-\u9fa5]/.test(result) ? "中文简体" : "英文";
        // Auto switch target language
        if (detectedLang === "中文简体") {
          targetLang = "en";
        } else {
          targetLang = "zh-CN";
        }
        await startTranslation(result);
      } else {
        ocrResult = "";
      }
    } catch (err) {
      ocrResult = `发生错误: ${err}`;
    } finally {
      isProcessing = false;
    }
  }

  async function startTranslation(text: string) {
    if (!text || !text.trim()) return;
    isTranslating = true;
    try {
      const result = await invoke<string>("translate_text", { text, targetLang });
      translatedText = result;
    } catch (err) {
      translatedText = `翻译错误: ${err}`;
    } finally {
      isTranslating = false;
    }
  }

  async function handleCancel() {
    isCapturing = false;
    document.body.style.backgroundColor = "";
    await appWindow.setFullscreen(false);
    captureBg = null;
  }

  function copyText(text: string | null) {
    if (!text) return;
    navigator.clipboard.writeText(text);
  }

  function dragWindow(e: MouseEvent) {
    const target = e.target as HTMLElement;
    // Don't drag if clicking on a button or its children
    if (target.closest('.tool-btn') || target.closest('button')) {
      return;
    }
    appWindow.startDragging();
  }

  async function hideWindow() {
    await appWindow.hide();
  }


  onMount(() => {
    // This is redundant but keeping for effect if needed
  });
  
  // $effect(() => {
  //   if (ocrResult && ocrResult.trim() && !isProcessing && !isTranslating) {
  //     startTranslation(ocrResult);
  //   }
  // });
  
  let translateTimer: any;
  function handleOcrInput() {
    if (translateTimer) clearTimeout(translateTimer);
    translateTimer = setTimeout(() => {
      if (ocrResult && ocrResult.trim()) {
        startTranslation(ocrResult);
      }
    }, 800);
  }

  async function togglePin() {
    isPinned = await invoke<boolean>("toggle_pin");
  }

  function toggleTargetLang() {
    targetLang = targetLang === "zh-CN" ? "en" : "zh-CN";
  }

</script>

{#if isCapturing}
  <CaptureOverlay backgroundImage={captureBg} onSelect={handleAreaSelect} onCancel={handleCancel} />
{/if}

{#if showSettings}
  <Settings onClose={() => toggleSettings(false)} />
{/if}

<main class="bob-container" style:display={isCapturing ? 'none' : 'block'}>
  <!-- Top Toolbar -->
  <header class="toolbar" onmousedown={dragWindow}>
    <div class="left-actions">
      <button class="tool-btn pin-btn" class:active={isPinned} onclick={togglePin} title={isPinned ? "取消固定" : "固定"}>
        <svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M16,12V4H17V2H7V4H8V12L6,14V16H11.2V22L12,22.8L12.8,22V16H18V14L16,12Z"/></svg>
      </button>
      <button class="tool-btn settings-btn" onclick={() => toggleSettings(true)} title="设置">
        <svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M19.14,12.94C19.14,12.78 19.14,12.61 19.14,12.45C19.14,12.29 19.14,12.12 19.14,11.96L21.5,10.12C21.71,9.96 21.77,9.67 21.63,9.44L19.39,5.55C19.25,5.32 18.96,5.23 18.72,5.33L15.94,6.45C15.35,6 14.73,5.63 14.07,5.36L13.65,2.41C13.61,2.15 13.38,1.96 13.11,1.96H8.62C8.35,1.96 8.13,2.15 8.09,2.41L7.67,5.35C7.01,5.62 6.39,6 5.8,6.45L3.02,5.33C2.78,5.23 2.49,5.32 2.35,5.55L0.11,9.44C-0.03,9.67 0.03,9.96 0.24,10.12L2.6,11.96C2.6,12.12 2.6,12.29 2.6,12.45C2.6,12.61 2.6,12.78 2.6,12.94L0.24,14.78C0.03,14.94 -0.03,15.23 0.11,15.46L2.35,19.35C2.49,19.58 2.78,19.67 3.02,19.57L5.8,18.45C6.39,18.9 7.01,19.28 7.67,19.55L8.09,22.49C8.13,22.75 8.35,22.94 8.62,22.94H13.11C13.38,22.94 13.61,22.75 13.65,22.49L14.07,19.55C14.73,19.28 15.35,18.9 15.94,18.45L18.72,19.57C18.96,19.67 19.25,19.58 19.39,19.35L21.63,15.46C21.77,15.23 21.71,14.94 21.5,14.78L19.14,12.94M10.87,12.45C10.87,13.79 9.77,14.89 8.43,14.89C7.09,14.89 6,13.79 6,12.45C6,11.11 7.09,10.02 8.43,10.02C9.77,10.02 10.87,11.11 10.87,12.45Z"/></svg>
      </button>
    </div>
    <div class="right-actions">

      <button class="tool-btn close-btn" onclick={hideWindow} title="隐藏">
        <svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/></svg>
      </button>
    </div>
  </header>

  <div class="content">
    <!-- Main Result Area -->
    <div class="result-scroll">
      <section class="result-card main-ocr">
        <div class="result-header">
           <div class="detected-lang-pill">
            <span class="label">识别为</span>
            <span class="value">{detectedLang}</span>
           </div>
           <button class="action-icon-btn" onclick={() => copyText(ocrResult)}><svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/></svg></button>
        </div>
        <div class="result-body">
          <textarea 
            class="ocr-text" 
            bind:value={ocrResult} 
            oninput={handleOcrInput}
            placeholder={isProcessing ? "正在识别中..." : "等待截图识别..."}
          ></textarea>
        </div>
      </section>

      <div class="divider">
        <div class="divider-line"></div>
        <button class="lang-switch-btn" onclick={toggleTargetLang}>
          {targetLang === "zh-CN" ? "译为中文" : "译为英文"}
          <svg viewBox="0 0 24 24" width="14" height="14"><path fill="currentColor" d="M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z"/></svg>
        </button>
        <div class="divider-line"></div>
      </div>

      <section class="result-card translation-result">
        <div class="result-header">
          <div class="target-lang-pill">
            <svg viewBox="0 0 24 24" width="14" height="14"><path fill="currentColor" d="M12.87,15.07L10.33,12.56L10.36,12.53C12.1,10.59 13.34,8.36 14.07,6H17V4H10V2H8V4H1V6H12.17C11.5,7.92 10.44,9.75 9,11.35C8.07,10.32 7.3,9.19 6.69,8H4.69C5.42,9.63 6.42,11.17 7.67,12.56L2.58,17.58L4,19L9,14L12.11,17.11L12.87,15.07M18.5,10H16.5L12,22H14L15.12,19H19.87L21,22H23L18.5,10M15.88,17L17.5,12.67L19.12,17H15.88Z"/></svg>
            {targetLang === "zh-CN" ? "中文简体" : "英文"}
          </div>
          </div>
        <div class="result-body">
          {#if isTranslating && !translatedText}
            <div class="skeleton-line shimmer"></div>
          {:else}
            <p class="translated-text">{translatedText || (isTranslating ? "Translating..." : "等待结果...")}</p>
          {/if}
          <div class="result-actions">
            <button class="action-icon-btn"><svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M14,3.23V5.29C16.89,6.15 19,8.83 19,12C19,15.17 16.89,17.85 14,18.71V20.77C18.01,19.86 21,16.28 21,12C21,7.72 18.01,4.14 14,3.23M16.5,12C16.5,10.23 15.5,8.71 14,7.97V16.03C15.5,15.29 16.5,13.77 16.5,12M3,9V15H7L12,20V4L7,9H3Z"/></svg></button>
            <button class="action-icon-btn" onclick={() => copyText(translatedText)}><svg viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/></svg></button>
          </div>
        </div>
      </section>

      <!-- FAB for Capture -->
      <button class="fab-btn" onclick={startCapture} title="开始识别">
        <svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M3,3H9V5H5V9H3V3M21,3V9H19V5H15V3H21M3,21V15H5V19H9V21H3M21,21H15V19H19V15H21V21M12,12L15,10L18,12L15,14L12,12Z"/></svg>
      </button>
    </div>
  </div>
</main>

<style>
  :root {
    --bob-bg: #f5f5f7;
    --bob-card-bg: #ffffff;
    --bob-border: #efefef;
    --bob-text-main: #333333;
    --bob-text-dim: #999999;
    --bob-accent: #3b82f6;
    --bob-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
    --bob-radius: 16px;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      --bob-bg: #1c1c1e;
      --bob-card-bg: #2c2c2e;
      --bob-border: #3c3c3e;
      --bob-text-main: #f2f2f7;
      --bob-text-dim: #8e8e93;
      --bob-accent: #0a84ff;
    }
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: transparent;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
    /* Round corners for borderless */
    border-radius: var(--bob-radius);
  }

  .bob-container {
    width: 450px;
    height: 550px;
    background: var(--bob-bg);
    color: var(--bob-text-main);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    border: 1px solid var(--bob-border);
    border-radius: var(--bob-radius);
    box-shadow: var(--bob-shadow);
  }

  .toolbar {
    height: 44px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 12px;
    background: var(--bob-bg);
    border-bottom: 1px solid var(--bob-border);
    cursor: move;
  }

  .left-actions, .right-actions {
    display: flex;
    gap: 4px;
  }

  .tool-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--bob-text-dim);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tool-btn:hover {
    background: var(--bob-border);
    color: var(--bob-text-main);
  }

  .tool-btn.pin-btn.active {
    color: var(--bob-accent);
    background: var(--bob-border);
  }

  .close-btn:hover {
    color: #ef4444;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .result-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    scrollbar-width: none;
  }

  .result-scroll::-webkit-scrollbar {
    display: none;
  }

  .result-card {
    background: var(--bob-card-bg);
    border-radius: 12px;
    border: 1px solid var(--bob-border);
    overflow: hidden;
    transition: box-shadow 0.2s ease;
  }

  .result-card:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  }

  .result-header {
    height: 32px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 12px;
    background: rgba(0, 0, 0, 0.02);
    border-bottom: 1px dotted var(--bob-border);
  }

  .detected-lang-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: rgba(0, 0, 0, 0.05);
    border-radius: 6px;
    font-size: 12px;
  }

  @media (prefers-color-scheme: dark) {
    .detected-lang-pill {
      background: rgba(255, 255, 255, 0.1);
    }
  }

  .detected-lang-pill .label {
    color: var(--bob-text-dim);
    font-weight: 400;
  }

  .detected-lang-pill .value {
    color: var(--bob-accent);
    font-weight: 600;
  }

  .target-lang-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 500;
    color: var(--bob-accent);
  }

  .result-body {
    padding: 12px;
    position: relative;
  }

  .ocr-text, .translated-text {
    margin: 0;
    font-size: 14px;
    line-height: 1.6;
    word-break: break-all;
    white-space: pre-wrap;
  }

  textarea.ocr-text {
    width: 100%;
    min-height: 100px;
    background: transparent;
    border: none;
    resize: vertical;
    font-family: inherit;
    color: inherit;
    padding: 0;
    outline: none;
    display: block;
  }

  .translated-text {
    min-height: 48px;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 4px 0;
  }

  .divider-line {
    flex: 1;
    height: 1px;
    background: var(--bob-border);
  }

  .lang-switch-btn {
    background: var(--bob-card-bg);
    border: 1px solid var(--bob-border);
    border-radius: 16px;
    padding: 4px 12px;
    font-size: 11px;
    color: var(--bob-text-dim);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .lang-switch-btn:hover {
    color: var(--bob-accent);
    border-color: var(--bob-accent);
  }

  .result-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .action-icon-btn {
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--bob-text-dim);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border-radius: 4px;
  }

  .action-icon-btn:hover {
    background: var(--bob-border);
    color: var(--bob-accent);
  }

  .skeleton-line {
    height: 14px;
    background: var(--bob-border);
    border-radius: 4px;
    margin-bottom: 8px;
    width: 100%;
  }

  .shimmer {
    background: linear-gradient(90deg, var(--bob-border) 25%, var(--bob-bg) 50%, var(--bob-border) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .fab-btn {
    position: absolute;
    right: 20px;
    bottom: 20px;
    width: 48px;
    height: 48px;
    border-radius: 24px;
    background: var(--bob-accent);
    color: white;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 12px rgba(10, 132, 255, 0.3);
    cursor: pointer;
    transition: transform 0.2s, background 0.2s;
    z-index: 10;
  }

  .fab-btn:hover {
    transform: scale(1.1);
    background: #0070e0;
  }

  .fab-btn:active {
    transform: scale(0.95);
  }
</style>

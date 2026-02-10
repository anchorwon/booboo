<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";

  let imgSrc = $state("");
  const appWindow = getCurrentWindow();

  onMount(async () => {
    try {
        const label = appWindow.label;
        console.log("Fetching image for label:", label);
        imgSrc = await invoke("get_pin_image", { label });
    } catch (e) {
        console.error("Failed to fetch pin image:", e);
        // Optional: Retry logic if needed, but cache should be ready.
        setTimeout(async () => {
             try {
                const label = appWindow.label;
                imgSrc = await invoke("get_pin_image", { label });
             } catch(err) {
                 console.error("Retry failed:", err);
             }
        }, 500);
    }
  });

  async function handleClose(e: MouseEvent) {
    console.log("Pin window close button clicked");
    e.preventDefault();
    e.stopPropagation();
    try {
      await invoke("close_window");
    } catch (err) {
      console.error("Failed to close window via Rust:", err);
      // Fallback
      appWindow.close();
    }
  }

  async function handleDoubleClick(e: MouseEvent) {
    console.log("Pin window double-clicked or right-clicked");
    e.preventDefault();
    e.stopPropagation();
    try {
      await invoke("close_window");
    } catch (err) {
      appWindow.close();
    }
  }
</script>

<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="pin-container" 
  data-tauri-drag-region
  ondblclick={handleDoubleClick}
  oncontextmenu={(e) => { e.preventDefault(); handleDoubleClick(e); }}
>
  {#if imgSrc}
    <img src={imgSrc} alt="Pinned capture" data-tauri-drag-region />
    <button class="close-btn" onclick={handleClose} title="关闭 (双击也可关闭)">
      <svg viewBox="0 0 24 24" width="18" height="18"><path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/></svg>
    </button>
  {:else}
    <div class="loading" data-tauri-drag-region>
      Loading...
      <button class="close-btn permanent" onclick={handleClose} title="关闭">
        <svg viewBox="0 0 24 24" width="18" height="18"><path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/></svg>
      </button>
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
    user-select: none;
  }

  .pin-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    position: relative;
    border: 1px solid rgba(255, 255, 255, 0.1);
    cursor: move;
  }

  .pin-container:hover .close-btn {
    opacity: 1;
  }

  img {
    max-width: 100%;
    max-height: 100%;
    display: block;
    pointer-events: none;
  }

  .close-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 28px;
    height: 28px;
    background: rgba(0, 0, 0, 0.4);
    color: white;
    border: none;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s;
    cursor: pointer;
    z-index: 100;
    -webkit-app-region: no-drag;
    pointer-events: auto !important;
  }

  .close-btn.permanent {
    opacity: 1;
    background: rgba(255, 0, 0, 0.6);
  }

  .close-btn svg {
    pointer-events: none;
  }

  .close-btn:hover {
    background: rgba(0, 0, 0, 0.9);
    transform: scale(1.1);
  }

  .loading {
    color: #666;
    font-size: 14px;
    font-family: sans-serif;
  }
</style>

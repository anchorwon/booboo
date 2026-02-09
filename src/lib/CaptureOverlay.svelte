<script>
  import { onMount } from 'svelte';

  let { backgroundImage = "", onSelect, onCancel } = $props();

  let startX = 0;
  let startY = 0;
  let currentX = 0;
  let currentY = 0;
  let isDragging = $state(false);
  let rect = $state({ x: 0, y: 0, width: 0, height: 0 });

  function handleMouseDown(e) {
    if (e.button === 2) { // Right click to cancel
      onCancel?.();
      return;
    }
    if (e.button !== 0) return;
    startX = e.clientX;
    startY = e.clientY;
    isDragging = true;
    updateRect(e);
  }

  function handleMouseMove(e) {
    if (!isDragging) return;
    updateRect(e);
  }

  function handleMouseUp(e) {
    if (!isDragging) return;
    isDragging = false;
    if (rect.width > 5 && rect.height > 5) {
      onSelect?.(rect);
    }
  }

  function updateRect(e) {
    currentX = e.clientX;
    currentY = e.clientY;
    
    rect.x = Math.min(startX, currentX);
    rect.y = Math.min(startY, currentY);
    rect.width = Math.max(1, Math.abs(currentX - startX));
    rect.height = Math.max(1, Math.abs(currentY - startY));
  }

  function handleKeyDown(e) {
    if (e.key === 'Escape') {
      onCancel?.();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });

  function handleContextMenu(e) {
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="overlay" 
  onmousedown={handleMouseDown} 
  onmousemove={handleMouseMove} 
  onmouseup={handleMouseUp}
  oncontextmenu={handleContextMenu}
>
  {#if backgroundImage && backgroundImage.length > 10}
    <!-- svelte-ignore a11y_missing_attribute -->
    <img 
      src={backgroundImage} 
      class="bg-img" 
      onerror={(e) => e.currentTarget.style.display = 'none'}
    />
  {/if}
  
  <div class="dimmer"></div>

  <div 
    class="selection" 
    style:left="{rect.x}px" 
    style:top="{rect.y}px" 
    style:width="{rect.width}px" 
    style:height="{rect.height}px"
  >
    {#if backgroundImage}
       <div 
         class="selection-bg"
         style:background-image="url({backgroundImage})"
         style:background-position="-{rect.x}px -{rect.y}px"
         style:background-size="100vw 100vh"
       ></div>
    {/if}
    
    {#if rect.width > 0}
      <div class="info">
        {rect.width} x {rect.height}
      </div>
    {/if}
    
    <div class="corner tl"></div>
    <div class="corner tr"></div>
    <div class="corner bl"></div>
    <div class="corner br"></div>
  </div>
  
  <!-- Hint container removed as per user feedback -->
  <!-- <div class="hint-container">
    <div class="hint">
      <span>拖拽选择区域，ESC 或 右键 取消</span>
      <button class="exit-btn" onclick={(e) => { e.stopPropagation(); onCancel?.(); }}>退出</button>
    </div>
  </div> -->
</div>

<style>
  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    cursor: crosshair;
    z-index: 9999;
    user-select: none;
    overflow: hidden;
  }

  .bg-img {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .dimmer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0);
  }

  .selection {
    position: absolute;
    border: 1px solid #3b82f6;
    box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.1);
    pointer-events: none;
    overflow: hidden;
  }

  .selection-bg {
    width: 100%;
    height: 100%;
    background-repeat: no-repeat;
  }

  .corner {
    position: absolute;
    width: 8px;
    height: 8px;
    background: #3b82f6;
  }
  .tl { top: -2px; left: -2px; }
  .tr { top: -2px; right: -2px; }
  .bl { bottom: -2px; left: -2px; }
  .br { bottom: -2px; right: -2px; }

  .info {
    position: absolute;
    top: -25px;
    left: 0;
    background: #3b82f6;
    color: white;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 2px;
    white-space: nowrap;
  }

  .hint-container {
    position: absolute;
    top: 30px;
    left: 0;
    width: 100%;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .hint {
    background: rgba(0, 0, 0, 0.7);
    color: white;
    padding: 8px 16px;
    border-radius: 20px;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    gap: 12px;
    pointer-events: auto;
  }

  .exit-btn {
    background: #ef4444;
    color: white;
    border: none;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .exit-btn:hover {
    background: #dc2626;
  }
</style>

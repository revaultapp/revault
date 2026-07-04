<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { formatBytes } from "$lib/utils";
  import { t } from "$lib/stores/locale.svelte";

  let { beforePath, afterPath, beforeSize, afterSize, onclose }: {
    beforePath: string;
    afterPath: string;
    beforeSize: number;
    afterSize: number;
    onclose: () => void;
  } = $props();

  let position = $state(50);
  let dragging = $state(false);
  let container: HTMLDivElement | undefined = $state();

  function updatePosition(clientX: number) {
    if (!container) return;
    const rect = container.getBoundingClientRect();
    position = Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
  }

  function onpointerdown(e: PointerEvent) {
    dragging = true;
    container?.setPointerCapture(e.pointerId);
    updatePosition(e.clientX);
  }

  function onpointermove(e: PointerEvent) {
    if (dragging) updatePosition(e.clientX);
  }

  function onpointerup() { dragging = false; }

  let beforeSrc = $derived(convertFileSrc(beforePath));
  let afterSrc = $derived(convertFileSrc(afterPath));
  let savings = $derived(beforeSize > 0 ? Math.round(((beforeSize - afterSize) / beforeSize) * 100) : 0);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />
<div class="overlay" role="dialog" aria-modal="true" aria-label={t("beforeAfterSlider.dialogAriaLabel")}>
  <div class="slider-card">
    <div class="slider-header">
      <span class="slider-title">{t("beforeAfterSlider.title")}</span>
      <span class="slider-savings">{t("beforeAfterSlider.savings", { pct: savings })}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <button class="slider-close" onclick={onclose} autofocus>{t("beforeAfterSlider.close")}</button>
    </div>
    <div class="slider-container" bind:this={container}
      role="slider" tabindex="0"
      aria-valuenow={Math.round(position)} aria-valuemin={0} aria-valuemax={100}
      aria-label={t("beforeAfterSlider.sliderAriaLabel")}
      onpointerdown={onpointerdown} onpointermove={onpointermove} onpointerup={onpointerup} onpointercancel={onpointerup}
      onkeydown={(e) => {
        if (e.key === 'ArrowLeft') { e.preventDefault(); position = Math.max(0, position - 5); }
        else if (e.key === 'ArrowRight') { e.preventDefault(); position = Math.min(100, position + 5); }
      }}>
      <img src={beforeSrc} alt={t("beforeAfterSlider.originalAlt")} class="slider-img" draggable="false" />
      <img src={afterSrc} alt={t("beforeAfterSlider.processedAlt")} class="slider-img" draggable="false"
        style="clip-path: inset(0 {100 - position}% 0 0);" />
      <div class="slider-divider" style="left: {position}%;">
        <div class="slider-handle"></div>
      </div>
      <span class="slider-label left">{formatBytes(beforeSize)}</span>
      <span class="slider-label right">{formatBytes(afterSize)}</span>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay-scrim);
    backdrop-filter: blur(4px);
  }

  .slider-card {
    background: var(--bg-card);
    border-radius: var(--radius-xl);
    border: 1px solid var(--border);
    padding: 16px;
    max-width: 800px;
    width: 90vw;
    box-shadow: var(--shadow-lg);
  }

  .slider-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .slider-title { font-size: 15px; font-weight: 600; }
  .slider-savings { font-size: 13px; color: var(--accent); font-weight: 500; }

  .slider-close {
    margin-left: auto;
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-muted);
    border: 1px solid var(--border);
    cursor: pointer;
  }
  .slider-close:hover { color: var(--text-primary); }

  .slider-container {
    position: relative;
    width: 100%;
    height: 400px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    cursor: col-resize;
    user-select: none;
    -webkit-user-select: none;
    background: var(--navy-bg);
  }

  .slider-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
  }

  .slider-divider {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent);
    transform: translateX(-50%);
    z-index: 2;
  }

  .slider-handle {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--bg-card);
    border: 2px solid var(--accent);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  .slider-label {
    position: absolute;
    top: 8px;
    z-index: 3;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    background: var(--overlay-scrim);
    color: var(--text-on-accent);
  }
  .slider-label.left { left: 8px; }
  .slider-label.right { right: 8px; }
</style>

<script lang="ts">
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import { Upload } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { IMAGE_EXTENSIONS, IMAGE_EXTENSIONS_RE } from "$lib/types";
  import { t } from "$lib/stores/locale.svelte";

  let {
    onfiles,
    acceptedExtensions = IMAGE_EXTENSIONS_RE,
    formatTags = [
      t("common.formatJpeg"), t("common.formatPng"), t("common.formatWebp"),
      t("common.formatHeic"), t("common.formatTiff"), t("common.formatBmp"),
      t("common.formatAvif"), t("common.formatJxl"),
    ],
    filePickerName = t("dropZone.filePickerName"),
    filePickerExtensions = [...IMAGE_EXTENSIONS] as string[],
    dropTitle = t("dropZone.dropTitle"),
    onrejectedfiles,
  }: {
    onfiles: (paths: string[]) => void;
    acceptedExtensions?: RegExp;
    formatTags?: string[];
    filePickerName?: string;
    filePickerExtensions?: string[];
    dropTitle?: string;
    onrejectedfiles?: (paths: string[]) => void;
  } = $props();

  let isDragging = $state(false);
  let isDropped = $state(false);
  let isInvalid = $state(false);
  let dropTimer: ReturnType<typeof setTimeout>;
  let invalidTimer: ReturnType<typeof setTimeout>;
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        const paths: string[] = [];
        const rejected: string[] = [];
        for (const path of event.payload.paths) {
          acceptedExtensions.lastIndex = 0;
          if (acceptedExtensions.test(path)) {
            paths.push(path);
          } else {
            rejected.push(path);
          }
        }
        if (rejected.length > 0) onrejectedfiles?.(rejected);
        if (paths.length > 0) {
          isInvalid = false;
          clearTimeout(invalidTimer);
          onfiles(paths);
          isDropped = true;
          clearTimeout(dropTimer);
          dropTimer = setTimeout(() => { isDropped = false; }, 600);
        } else {
          isDropped = false;
          clearTimeout(dropTimer);
          isInvalid = true;
          clearTimeout(invalidTimer);
          invalidTimer = setTimeout(() => { isInvalid = false; }, 400);
        }
      } else {
        isDragging = false;
      }
    });
  });

  onDestroy(() => unlisten?.());

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: filePickerName, extensions: filePickerExtensions }],
    });
    if (selected) onfiles(selected);
  }
</script>

<div
  class="empty"
  class:dragging={isDragging}
  class:dropped={isDropped}
  class:invalid={isInvalid}
  role="button"
  tabindex="0"
  onclick={browseFiles}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      browseFiles();
    }
  }}
>
  <div class="drop-zone">
    <svg
      class="border-svg"
      width="100%"
      height="100%"
      style="position:absolute;inset:0;pointer-events:none;overflow:visible"
    >
      <rect
        class="border-rect"
        x="1"
        y="1"
        width="calc(100% - 2px)"
        height="calc(100% - 2px)"
        rx="15"
        ry="15"
        fill="none"
        stroke="var(--border)"
        stroke-width="2"
        stroke-dasharray="8 6"
      />
    </svg>
    <Upload class="upload-icon" size={40} strokeWidth={1.5} />
    <p class="drop-title">{dropTitle}</p>
    <p class="drop-sub">{t("dropZone.dropSubtitle")}</p>
    <div class="format-tags">
      {#each formatTags as tag (tag)}
        <span class="tag">{tag}</span>
      {/each}
    </div>
    {#if isInvalid}
      <p class="invalid-msg" transition:fade={{ duration: 200 }}>{t("dropZone.formatNotSupported")}</p>
    {/if}
  </div>
</div>

<style>
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    cursor: pointer;
  }

  .drop-zone {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    min-width: min(480px, 100%);
    padding: 88px 112px;
    border-radius: 16px;
    color: var(--text-muted);
    transition:
      transform 150ms var(--ease-out),
      box-shadow 200ms var(--ease-out),
      color var(--duration-normal) var(--ease-out);
  }

  /* SVG border rect */
  .border-rect {
    transition: stroke var(--duration-normal) var(--ease-out);
    animation: dash-march 0.8s linear infinite;
  }

  @keyframes dash-march {
    to { stroke-dashoffset: -14; }
  }

  /* Pause march + accent on hover or drag */
  .empty:hover .border-rect,
  .empty.dragging .border-rect {
    animation-play-state: paused;
    stroke: var(--accent);
  }

  /* Invalid border */
  .empty.invalid .border-rect {
    animation-play-state: paused;
    stroke: var(--danger);
  }

  /* Hover color */
  .empty:hover .drop-zone {
    color: var(--accent);
  }

  /* Drag-over magnético */
  .empty.dragging .drop-zone {
    transform: scale(1.02);
    box-shadow: 0 0 0 4px var(--accent-glow), 0 8px 32px rgba(16, 216, 122, 0.12);
    color: var(--accent);
  }

  /* Upload icon hover/drag — upload-icon class only, not border-svg */
  .empty:hover .drop-zone :global(.upload-icon),
  .empty.dragging .drop-zone :global(.upload-icon) {
    transform: translateY(-3px) scale(1.1);
    transition: transform 200ms var(--ease-out);
    color: var(--accent);
  }

  /* Drop confirm animation */
  .empty.dropped .drop-zone {
    animation: drop-confirm 600ms var(--ease-out) forwards;
  }

  @keyframes drop-confirm {
    0%   { transform: scale(0.98); }
    30%  { transform: scale(1.03); }
    60%  { transform: scale(1.01); }
    100% { transform: scale(1); }
  }

  /* Ring expansivo on drop */
  .empty.dropped .drop-zone::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 16px;
    border: 2px solid var(--accent);
    animation: ring-expand 600ms var(--ease-out) forwards;
    pointer-events: none;
  }

  @keyframes ring-expand {
    0%   { transform: scale(1); opacity: 0.8; }
    100% { transform: scale(1.06); opacity: 0; }
  }

  /* Shake on invalid */
  .empty.invalid .drop-zone {
    animation: shake 400ms var(--ease-out);
    box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.12);
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    20%       { transform: translateX(-5px); }
    40%       { transform: translateX(5px); }
    60%       { transform: translateX(-3px); }
    80%       { transform: translateX(3px); }
  }

  /* Invalid message */
  .invalid-msg {
    position: absolute;
    bottom: 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--danger);
  }

  .drop-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .drop-sub {
    font-size: 13px;
    color: var(--text-muted);
  }

  .format-tags {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .tag {
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    background: var(--navy-bg);
    color: var(--text-secondary);
  }

  @media (prefers-reduced-motion: reduce) {
    .border-rect {
      animation: none;
    }

    .empty.dropped .drop-zone,
    .empty.dropped .drop-zone::after,
    .empty.invalid .drop-zone {
      animation: none;
    }

    .drop-zone,
    .empty:hover .drop-zone :global(.upload-icon),
    .empty.dragging .drop-zone :global(.upload-icon) {
      transition: none;
    }
  }
</style>

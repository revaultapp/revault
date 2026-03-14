<script lang="ts">
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import { Upload } from "lucide-svelte";

  let {
    onfiles,
    acceptedExtensions = /\.(jpe?g|png|webp|heic|heif|tiff?|bmp|gif)$/i,
    formatTags = ["JPEG", "PNG", "WebP", "HEIC", "TIFF", "BMP", "GIF"],
  }: {
    onfiles: (paths: string[]) => void;
    acceptedExtensions?: RegExp;
    formatTags?: string[];
  } = $props();

  let isDragging = $state(false);
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        const paths = event.payload.paths.filter((p) => acceptedExtensions.test(p));
        if (paths.length > 0) onfiles(paths);
      } else {
        isDragging = false;
      }
    });
  });

  onDestroy(() => unlisten?.());

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "bmp", "gif"] }],
    });
    if (selected) onfiles(selected);
  }
</script>

<div class="empty" class:dragging={isDragging} role="button" tabindex="0" onclick={browseFiles} onkeydown={(e) => e.key === "Enter" && browseFiles()}>
  <div class="drop-zone">
    <Upload size={40} strokeWidth={1.5} />
    <p class="drop-title">Drop images here</p>
    <p class="drop-sub">or click to browse</p>
    <div class="format-tags">
      {#each formatTags as tag}
        <span class="tag">{tag}</span>
      {/each}
    </div>
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
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 80px;
    border: 2px dashed var(--border);
    border-radius: 16px;
    color: var(--text-muted);
    transition: border-color 0.2s, color 0.2s;
  }

  .empty:hover .drop-zone,
  .empty.dragging .drop-zone {
    border-color: var(--accent);
    color: var(--accent);
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
</style>

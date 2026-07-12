<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fade, slide, scale } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import { Trash2, FolderOpen, Search, ChevronDown, ChevronRight, FolderSearch, CircleX, Layers, HardDrive, ImageIcon } from "lucide-svelte";
  import ProgressRing from "./ProgressRing.svelte";
  import Button from "./Button.svelte";
  import SegmentedControl from "./SegmentedControl.svelte";
  import { duplicateGroups, isScanning, totalFound, scanError, scanForDuplicates, cancelScan, clearResults, scanProgress, scanMode, setMode } from "$lib/stores/dedupe";
  import { formatBytes } from "$lib/utils";
  import { activity } from "$lib/stores/activity";
  import { t } from "$lib/stores/locale.svelte";

  let modeSegments = $derived([
    { id: "exact", label: t("analyze.modeExact") },
    { id: "similar", label: t("analyze.modeSimilar") },
  ] as const);

  let modeSelection = $state<string>($scanMode);

  $effect(() => {
    modeSelection = $scanMode;
  });

  let modeDescription = $derived(
    modeSelection === "similar" ? t("analyze.modeDescSimilar") : t("analyze.modeDescExact")
  );

  let modeDescriptionDetail = $derived(
    modeSelection === "similar" ? t("analyze.modeDescSimilarDetail") : t("analyze.modeDescExactDetail")
  );

  const rm = $derived(prefersReducedMotion.current);

  let selectedFolders = $state<string[]>([]);
  let expandedGroups = $state<Set<string>>(new Set());
  let deletedPaths = $state<Set<string>>(new Set());
  let isDragging = $state(false);
  let thumbnails = $state<Record<string, string>>({});

  onMount(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        const paths = event.payload.paths;
        // Filter to only paths that don't look like image files (Analyze expects folders)
        const newFolders = paths.filter((p: string) => {
          if (selectedFolders.includes(p)) return false;
          const ext = p.split('.').pop()?.toLowerCase();
          const imageExts = ['jpg', 'jpeg', 'png', 'webp', 'heic', 'heif', 'tiff', 'tif', 'bmp', 'gif', 'avif', 'jxl'];
          return !ext || !imageExts.includes(ext);
        });
        if (newFolders.length > 0) {
          selectedFolders = [...selectedFolders, ...newFolders];
          clearResults();
          deletedPaths = new Set();
          expandedGroups = new Set();
        }
      } else {
        isDragging = false;
      }
    });
    return () => {
      unlisten.then((fn: () => void) => fn());
    };
  });

  let totalWastedSpace = $derived(
    $duplicateGroups.reduce((acc, group) => {
      const sorted = [...group.files].sort((a, b) => b.size - a.size);
      const originals = sorted.slice(1);
      return acc + originals.reduce((a, f) => a + f.size, 0);
    }, 0)
  );

  let visibleGroups = $derived(
    $duplicateGroups.filter((g) =>
      g.files.some((f) => !deletedPaths.has(f.path))
    )
  );

  let visibleFileCount = $derived(
    visibleGroups.reduce((acc, g) => acc + g.files.length, 0)
  );

  // Lazily fetch thumbnails only for files inside expanded groups (collapsed
  // groups don't need one yet). Same stale-response guard as ToolShell.
  $effect(() => {
    const submitted = new Set<string>();
    for (const group of visibleGroups) {
      if (!expandedGroups.has(group.hash)) continue;
      for (const file of group.files) {
        if (thumbnails[file.path] !== undefined) continue;
        thumbnails[file.path] = ""; // mark as loading
        submitted.add(file.path);
        invoke<string>("generate_thumbnail", { path: file.path })
          .then((src) => {
            if (submitted.has(file.path)) {
              thumbnails[file.path] = src;
            }
          })
          .catch(() => {
            if (submitted.has(file.path)) {
              thumbnails[file.path] = "error";
            }
          });
      }
    }
  });

  let progressPct = $derived(
    $scanProgress && $scanProgress.total > 0
      ? Math.round(($scanProgress.current / $scanProgress.total) * 100)
      : 0
  );

  let ringPct = $derived(
    $scanProgress?.phase === "grouping" ? 100 : progressPct
  );

  async function browseFolders() {
    const selected = await open({
      directory: true,
      multiple: true,
    });
    if (selected) {
      const folders = Array.isArray(selected) ? selected : [selected];
      const newFolders = folders.filter((f) => !selectedFolders.includes(f));
      selectedFolders = [...selectedFolders, ...newFolders];
      if (newFolders.length > 0) {
        clearResults();
        deletedPaths = new Set();
        expandedGroups = new Set();
      }
    }
  }

  function removeFolder(path: string) {
    selectedFolders = selectedFolders.filter((f) => f !== path);
    clearResults();
    deletedPaths = new Set();
    expandedGroups = new Set();
  }

  async function startScan() {
    if (selectedFolders.length === 0) return;
    deletedPaths = new Set();
    expandedGroups = new Set();
    await scanForDuplicates(selectedFolders);
    if ($totalFound > 0) {
      activity.add({ type: "analyze", fileCount: $totalFound, savedBytes: 0 });
    }
  }

  function handleModeSelect(mode: string) {
    setMode(mode as "exact" | "similar");
  }

  async function stopScan() {
    await cancelScan();
  }

  function toggleGroup(hash: string) {
    if (expandedGroups.has(hash)) {
      expandedGroups.delete(hash);
    } else {
      expandedGroups.add(hash);
    }
    expandedGroups = new Set(expandedGroups);
  }

  async function deleteDuplicate(filePath: string) {
    if ($scanMode === "similar") {
      const ok = window.confirm(t("analyze.similarFileConfirm"));
      if (!ok) return;
    }
    try {
      const results = await invoke<{ path: string; success: boolean; error?: string }[]>("delete_files", { paths: [filePath] });
      const result = results[0];
      if (result.success) {
        deletedPaths = new Set([...deletedPaths, filePath]);
      } else {
        scanError.set(result.error ?? t("analyze.failedToDeleteFile"));
      }
    } catch (e) {
      scanError.set(String(e));
    }
  }

  function clearAll() {
    selectedFolders = [];
    clearResults();
    deletedPaths = new Set();
    expandedGroups = new Set();
  }

  function folderName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }
</script>

{#if selectedFolders.length === 0}
  <div class="empty-view">
    <div class="folder-drop-zone" class:dragging={isDragging}>
      <div class="drop-icon">
        <FolderSearch size={48} strokeWidth={1.5} />
      </div>
      <h2 class="drop-title">{t("analyze.dropTitle")}</h2>
      <p class="drop-subtitle">{t("analyze.dropSubtitle")}</p>

      <!-- Decorative — the same flow is already spelled out in the title,
           subtitle, and action button below, so it's redundant for AT users. -->
      <div class="flow-diagram" aria-hidden="true">
        <span class="flow-step">
          <Search size={14} class="flow-icon flow-icon--blue" />
          {t("analyze.flowStepScan")}
        </span>
        <ChevronRight size={12} class="flow-arrow" />
        <span class="flow-step">
          <Layers size={14} class="flow-icon flow-icon--violet" />
          {t("analyze.flowStepGroup")}
        </span>
        <ChevronRight size={12} class="flow-arrow" />
        <span class="flow-step">
          <HardDrive size={14} class="flow-icon flow-icon--amber" />
          {t("analyze.flowStepReclaim")}
        </span>
      </div>

      <SegmentedControl segments={modeSegments} bind:selected={modeSelection} onselect={handleModeSelect} label={t("analyze.matchModeAriaLabel")} />
      {#key modeSelection}
        <div
          class="mode-copy"
          in:fade={{ duration: rm ? 0 : 150, easing: cubicOut }}
          out:fade={{ duration: rm ? 0 : 100, easing: cubicOut }}
        >
          <p class="mode-description">{modeDescription}</p>
          <p class="mode-detail">{modeDescriptionDetail}</p>
        </div>
      {/key}
      <Button onclick={browseFolders} aria-label={t("analyze.chooseFoldersAriaLabel")}>
        <FolderOpen size={16} />
        {t("analyze.chooseFolders")}
      </Button>
    </div>
  </div>
{:else if $isScanning}
  <div class="scanning-view">
    <ProgressRing
      targetPct={ringPct}
      label={$scanProgress?.phase === "grouping" ? t("analyze.groupingLabel") : t("analyze.scanningFilesLabel")}
      sublabel={$scanProgress
        ? t("analyze.scanProgressSublabel", { current: $scanProgress.current, total: $scanProgress.total })
        : (selectedFolders.length === 1
          ? t("analyze.folderCountOne", { count: selectedFolders.length })
          : t("analyze.folderCountOther", { count: selectedFolders.length }))}
    />
    <div class="scanning-actions">
      <Button variant="ghost" danger onclick={stopScan} aria-label={t("analyze.cancelScanAriaLabel")}>
        <CircleX size={14} />
        {t("analyze.cancel")}
      </Button>
    </div>
  </div>
{:else if $duplicateGroups.length === 0}
  <div class="empty-view">
    <div class="no-dupes">
      <Search size={40} strokeWidth={1.5} />
      <p class="no-dupes-title">{t("analyze.noDuplicatesTitle")}</p>
      <p class="no-dupes-sub">{t("analyze.noDuplicatesSub")}</p>
      <div class="folders-added">
        {#each selectedFolders as folder (folder)}
          <span class="folder-chip">
            <FolderOpen size={12} />
            {folderName(folder)}
            <button class="chip-remove" onclick={() => removeFolder(folder)} aria-label={t("analyze.removeFolderAriaLabel", { name: folderName(folder) })}>×</button>
          </span>
        {/each}
      </div>
      <Button onclick={startScan}>
        <Search size={14} />
        {t("analyze.rescan")}
      </Button>
    </div>
  </div>
{:else}
  <div class="results-view">
    {#if $scanError}
      <div class="error-banner" role="alert">
        <span class="error-text">{$scanError}</span>
        <button class="error-dismiss" onclick={() => scanError.set(null)} aria-label={t("analyze.dismissErrorAriaLabel")}>×</button>
      </div>
    {/if}
    <div class="header">
      <div class="header-left">
        <div class="title-row">
          <h2>
            {#if $scanMode === "similar"}
              {visibleGroups.length === 1
                ? t("analyze.groupsFoundSimilarOne", { count: visibleGroups.length })
                : t("analyze.groupsFoundSimilarOther", { count: visibleGroups.length })}
            {:else}
              {visibleGroups.length === 1
                ? t("analyze.groupsFoundDuplicateOne", { count: visibleGroups.length })
                : t("analyze.groupsFoundDuplicateOther", { count: visibleGroups.length })}
            {/if}
          </h2>
          <span class="mode-tag">{$scanMode === "similar" ? t("analyze.modeTagSimilar") : t("analyze.modeTagExact")}</span>
        </div>
        <span class="sub">{t("analyze.filesWastedSummary", { count: visibleFileCount, size: formatBytes(totalWastedSpace) })}</span>
      </div>
      <div class="header-actions">
        <Button variant="ghost" onclick={browseFolders} aria-label={t("analyze.addMoreFoldersAriaLabel")}>
          <FolderOpen size={14} />
          {t("analyze.addFolders")}
        </Button>
        <Button variant="ghost" onclick={clearAll} aria-label={t("analyze.clearAllAriaLabel")}>
          <Trash2 size={14} />
          {t("analyze.clear")}
        </Button>
      </div>
    </div>

    <div class="folders-bar">
      <SegmentedControl segments={modeSegments} bind:selected={modeSelection} onselect={handleModeSelect} label={t("analyze.matchModeAriaLabel")} />
      {#each selectedFolders as folder (folder)}
        <span class="folder-chip">
          <FolderOpen size={12} />
          {folderName(folder)}
          <button class="chip-remove" onclick={() => removeFolder(folder)} aria-label={t("analyze.removeFolderAriaLabel", { name: folderName(folder) })}>×</button>
        </span>
      {/each}
      {#if selectedFolders.length > 0}
        <Button onclick={startScan} disabled={$isScanning}>
          <Search size={14} />
          {t("analyze.scan")}
        </Button>
      {/if}
    </div>

    <div class="groups-list">
      {#each visibleGroups as group (group.hash)}
        {@const sortedFiles = [...group.files].sort((a, b) => b.size - a.size)}
        {@const wasted = sortedFiles.slice(1).reduce((a, f) => a + f.size, 0)}
        {@const isExpanded = expandedGroups.has(group.hash)}
        <div
          class="group-card"
          animate:flip={{ duration: rm ? 0 : 200, easing: cubicOut }}
          out:scale={{ duration: rm ? 0 : 160, start: 0.96, opacity: 0, easing: cubicOut }}
        >
          <button class="group-header" onclick={() => toggleGroup(group.hash)} aria-expanded={isExpanded}>
            <div class="group-info">
              {#if isExpanded}
                <ChevronDown size={16} />
              {:else}
                <ChevronRight size={16} />
              {/if}
              {#if $scanMode === "similar"}
                <span class="similarity-badge">{t("analyze.similarityMatch", { pct: Math.round((1 - group.max_distance / 256) * 100) })}</span>
              {:else}
                <span class="similarity-badge" title={t("analyze.byteIdenticalTitle")}>{t("analyze.exactBadgeLabel")}</span>
              {/if}
              <span class="group-count">{t("analyze.filesCountLabel", { count: group.files.length })}</span>
            </div>
            <div class="group-meta">
              <span class="wasted-badge">{$scanMode === "similar" ? t("analyze.wastedRedundant", { size: formatBytes(wasted) }) : t("analyze.wastedDuplicate", { size: formatBytes(wasted) })}</span>
            </div>
          </button>

          {#if isExpanded}
            <div class="group-files" transition:slide={{ duration: rm ? 0 : 200, easing: cubicOut }}>
              {#each sortedFiles as file, fi (file.path)}
                {@const isDeleted = deletedPaths.has(file.path)}
                <div class="dup-file" class:original={fi === 0} class:deleted={isDeleted}>
                  {#if thumbnails[file.path] && thumbnails[file.path] !== "error"}
                    <img class="dup-thumb" src={thumbnails[file.path]} alt="" draggable="false" />
                  {:else}
                    <div class="dup-thumb placeholder">
                      <ImageIcon size={16} />
                    </div>
                  {/if}
                  <div class="dup-info">
                    <span class="dup-path" title={file.path}>{file.path.split(/[\\/]/).pop()}</span>
                    <span class="dup-folder">{file.path.split(/[\\/]/).slice(-2, -1)[0] ?? ""}</span>
                  </div>
                  <div class="dup-meta">
                    <span class="dup-size">{formatBytes(file.size)}</span>
                    {#if fi === 0}
                      {#if $scanMode === "similar"}
                        <span class="largest-tag">{t("analyze.largestTag")}</span>
                      {:else}
                        <span class="original-tag">{t("analyze.originalTag")}</span>
                      {/if}
                    {:else if !isDeleted}
                      <button class="btn-icon delete-btn" onclick={() => deleteDuplicate(file.path)} title={t("analyze.moveToTrashTitle")} aria-label={t("analyze.moveToTrashAriaLabel")}>
                        <Trash2 size={14} />
                      </button>
                    {:else}
                      <span class="deleted-tag">{t("analyze.markedDeletedTag")}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .empty-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 28px;
  }

  .folder-drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 64px 48px;
    background: var(--bg-card);
    border: 2px dashed var(--border);
    border-radius: var(--radius-xl);
    text-align: center;
    max-width: 400px;
    transition:
      transform var(--duration-normal) var(--ease-out),
      box-shadow var(--duration-normal) var(--ease-out),
      border-color var(--duration-normal) var(--ease-out);
  }

  /* Drag-over feedback — same language as DropZone.svelte's .dragging state. */
  .folder-drop-zone.dragging {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-glow), 0 8px 32px rgba(16, 216, 122, 0.12);
    transform: scale(1.02);
  }

  .drop-icon {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .folder-drop-zone.dragging .drop-icon {
    color: var(--accent);
    opacity: 1;
  }

  .drop-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .drop-subtitle {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: -8px;
  }

  .flow-diagram {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .flow-step {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.02em;
    color: var(--text-muted);
  }

  .flow-step :global(.flow-icon) {
    flex-shrink: 0;
  }

  .flow-step :global(.flow-icon--blue) { color: var(--cat-blue); }
  .flow-step :global(.flow-icon--violet) { color: var(--cat-violet); }
  .flow-step :global(.flow-icon--amber) { color: var(--cat-amber); }

  .flow-diagram :global(.flow-arrow) {
    color: var(--text-muted);
    opacity: 0.5;
    flex-shrink: 0;
  }

  .mode-description {
    max-width: 280px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  /* El detalle del método va como texto visible: el (i) con title nativo
     no renderiza tooltip en WKWebView (macOS), así que nunca se veía. */
  .mode-copy {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .mode-detail {
    max-width: 340px;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-muted);
    opacity: 0.8;
    text-wrap: balance;
  }

  .scanning-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
    height: 100%;
  }

  .scanning-actions {
    display: flex;
    justify-content: center;
  }

  .results-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px;
    height: 100%;
    overflow: hidden;
  }

  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--danger-bg);
    border: 1px solid var(--danger);
    border-radius: var(--radius-sm);
    color: var(--danger);
    font-size: 13px;
  }

  .error-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-dismiss {
    background: none;
    border: none;
    color: var(--danger);
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .header-left {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .header h2 {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .mode-tag {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
    background: var(--navy-bg);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .sub {
    font-size: 13px;
    color: var(--text-muted);
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .folders-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-card);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .folder-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--navy-bg);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .chip-remove {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0 0 0 2px;
  }

  .chip-remove:hover {
    color: var(--danger);
  }

  .groups-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .group-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .group-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .group-header:hover {
    background: var(--navy-bg);
  }

  .group-info {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-muted);
  }

  .similarity-badge {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--navy-bg);
    padding: 2px 8px;
    border-radius: 4px;
    font-variant-numeric: tabular-nums;
  }

  .group-count {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .wasted-badge {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-subtle);
    padding: 2px 8px;
    border-radius: 4px;
    font-variant-numeric: tabular-nums;
  }

  .group-files {
    border-top: 1px solid var(--border);
  }

  .dup-file {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    gap: 12px;
    transition: opacity var(--duration-normal) var(--ease-out);
  }

  .dup-file:last-child {
    border-bottom: none;
  }

  .dup-file.original {
    background: rgba(16, 216, 122, 0.04);
  }

  .dup-file.deleted {
    opacity: 0.4;
    text-decoration: line-through;
  }

  .dup-thumb {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border);
  }

  .dup-thumb.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--navy-bg);
    color: var(--text-muted);
  }

  .dup-file.original .dup-thumb {
    border-color: var(--accent);
  }

  .dup-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .dup-path {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dup-folder {
    font-size: 11px;
    color: var(--text-muted);
  }

  .dup-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .dup-size {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .original-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-subtle);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .largest-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--navy-bg);
    padding: 2px 8px;
    border-radius: 4px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .deleted-tag {
    font-size: 11px;
    color: var(--text-muted);
  }

  .delete-btn {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color 0.15s;
  }

  .delete-btn:hover {
    color: var(--danger);
  }

  .delete-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .delete-btn:disabled:hover {
    color: var(--text-muted);
  }

  .no-dupes {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: var(--text-muted);
    padding: 64px;
  }

  .no-dupes-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 8px;
  }

  .no-dupes-sub {
    font-size: 13px;
    color: var(--text-muted);
  }

  .folders-added {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
    margin: 8px 0;
  }

</style>

<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { Trash2, FolderOpen, Search, ChevronDown, ChevronRight } from "lucide-svelte";
  import DropZone from "./DropZone.svelte";
  import ProgressRing from "./ProgressRing.svelte";
  import { duplicateGroups, isScanning, totalFound, scanForDuplicates, clearResults } from "$lib/stores/dedupe";
  import { formatBytes } from "$lib/utils";
  import { activity } from "$lib/stores/activity";

  let selectedFolders = $state<string[]>([]);
  let expandedGroups = $state<Set<number>>(new Set());
  let deletedPaths = $state<Set<string>>(new Set());

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

  let visibleTotal = $derived(
    visibleGroups.reduce((acc, g) => acc + g.files.length, 0)
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
    }
  }

  function removeFolder(path: string) {
    selectedFolders = selectedFolders.filter((f) => f !== path);
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

  function toggleGroup(i: number) {
    if (expandedGroups.has(i)) {
      expandedGroups.delete(i);
    } else {
      expandedGroups.add(i);
    }
    expandedGroups = new Set(expandedGroups);
  }

  async function deleteDuplicate(filePath: string) {
    // Note: moving to OS trash requires tauri-plugin-fs which is not yet installed.
    // For now, mark as deleted in UI only. A future update will wire up the actual delete.
    deletedPaths = new Set([...deletedPaths, filePath]);
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

{#if selectedFolders.length === 0 && visibleGroups.length === 0}
  <div class="empty-view">
    <DropZone
      onfiles={(paths) => { selectedFolders = [...selectedFolders, ...paths.filter(p => !selectedFolders.includes(p))]; }}
      acceptedExtensions={/.*/}
      formatTags={["JPEG", "PNG", "WebP", "HEIC", "TIFF", "BMP", "AVIF", "JXL", "Folder"]}
    />
  </div>
{:else if $isScanning}
  <div class="scanning-view">
    <ProgressRing
      targetPct={100}
      label="Scanning for duplicates..."
      sublabel={`${selectedFolders.length} folder${selectedFolders.length > 1 ? "s" : ""} · ${$duplicateGroups.length} groups found`}
    />
  </div>
{:else if visibleGroups.length > 0}
  <div class="results-view">
    <div class="header">
      <div class="header-left">
        <h2>{visibleGroups.length} duplicate group{visibleGroups.length > 1 ? "s" : ""} found</h2>
        <span class="sub">{visibleTotal} files · {formatBytes(totalWastedSpace)} wasted</span>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" onclick={browseFolders}>
          <FolderOpen size={14} />
          Add folders
        </button>
        <button class="btn-ghost" onclick={clearAll}>
          <Trash2 size={14} />
          Clear
        </button>
      </div>
    </div>

    <div class="folders-bar">
      {#each selectedFolders as folder}
        <span class="folder-chip">
          <FolderOpen size={12} />
          {folderName(folder)}
          <button class="chip-remove" onclick={() => removeFolder(folder)}>×</button>
        </span>
      {/each}
      {#if selectedFolders.length > 0}
        <button class="btn-primary scan-btn" onclick={startScan}>
          <Search size={14} />
          Scan
        </button>
      {/if}
    </div>

    <div class="groups-list">
      {#each visibleGroups as group, i}
        {@const sortedFiles = [...group.files].sort((a, b) => b.size - a.size)}
        {@const wasted = sortedFiles.slice(1).reduce((a, f) => a + f.size, 0)}
        {@const isExpanded = expandedGroups.has(i)}
        <div class="group-card">
          <button class="group-header" onclick={() => toggleGroup(i)}>
            <div class="group-info">
              {#if isExpanded}
                <ChevronDown size={16} />
              {:else}
                <ChevronRight size={16} />
              {/if}
              <span class="group-hash">{group.hash.slice(0, 12)}</span>
              <span class="group-count">{group.files.length} files</span>
            </div>
            <div class="group-meta">
              <span class="wasted-badge">{formatBytes(wasted)} duplicate</span>
            </div>
          </button>

          {#if isExpanded}
            <div class="group-files">
              {#each sortedFiles as file, fi}
                {@const isDeleted = deletedPaths.has(file.path)}
                <div class="dup-file" class:original={fi === 0} class:deleted={isDeleted}>
                  <div class="dup-info">
                    <span class="dup-path" title={file.path}>{file.path.split(/[\\/]/).pop()}</span>
                    <span class="dup-folder">{file.path.split(/[\\/]/).slice(-2, -1)[0] ?? ""}</span>
                  </div>
                  <div class="dup-meta">
                    <span class="dup-size">{formatBytes(file.size)}</span>
                    {#if fi === 0}
                      <span class="original-tag">Original</span>
                    {:else if !isDeleted}
                      <button class="btn-icon delete-btn" onclick={() => deleteDuplicate(file.path)} title="Move to trash">
                        <Trash2 size={14} />
                      </button>
                    {:else}
                      <span class="deleted-tag">Marked deleted</span>
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
{:else}
  <div class="empty-view">
    <div class="no-dupes">
      <Search size={40} strokeWidth={1.5} />
      <p class="no-dupes-title">No duplicates found</p>
      <p class="no-dupes-sub">Try adding more folders or rescanning</p>
      <div class="folders-added">
        {#each selectedFolders as folder}
          <span class="folder-chip">
            <FolderOpen size={12} />
            {folderName(folder)}
            <button class="chip-remove" onclick={() => removeFolder(folder)}>×</button>
          </span>
        {/each}
      </div>
      <button class="btn-primary" onclick={startScan}>
        <Search size={14} />
        Rescan
      </button>
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

  .scanning-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .results-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px;
    height: 100%;
    overflow: hidden;
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

  .header h2 {
    font-size: 18px;
    font-weight: 600;
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
    color: #ef4444;
  }

  .scan-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 16px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .scan-btn:hover {
    opacity: 0.9;
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

  .group-hash {
    font-family: monospace;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--navy-bg);
    padding: 2px 6px;
    border-radius: 4px;
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
    background: rgba(16, 185, 129, 0.1);
    padding: 2px 8px;
    border-radius: 4px;
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
  }

  .dup-file:last-child {
    border-bottom: none;
  }

  .dup-file.original {
    background: rgba(16, 185, 129, 0.04);
  }

  .dup-file.deleted {
    opacity: 0.4;
    text-decoration: line-through;
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
  }

  .original-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: rgba(16, 185, 129, 0.1);
    padding: 2px 8px;
    border-radius: 4px;
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
    color: #ef4444;
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

  .btn-ghost {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    background: none;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-ghost:hover {
    background: var(--navy-bg);
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 28px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }
</style>

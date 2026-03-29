import { writable, derived } from "svelte/store";
import type { BaseFile } from "$lib/types";

export type OrganizeTab = "rename" | "organize";
export type RenameStatus = "pending" | "done" | "error";

export interface OrganizeFile extends BaseFile {
  status: RenameStatus;
  newPath?: string;
  error?: string;
}

export const activeTab = writable<OrganizeTab>("rename");

// Rename state
export const renameFiles = writable<OrganizeFile[]>([]);
export const template = writable("{name}_{counter}.{ext}");

// Organize state
export const sourceDir = writable<string | null>(null);
export const destDir = writable<string | null>(null);
export const organizeMode = writable<"copy" | "move">("copy");
export const isOrganizing = writable(false);
export const organizeResult = writable<{ moved: number; skipped: number; errors: string[] } | null>(null);

export const renameSummary = derived(renameFiles, ($files) => {
  const done = $files.filter((f) => f.status === "done").length;
  const failed = $files.filter((f) => f.status === "error").length;
  const pending = $files.filter((f) => f.status === "pending").length;
  return { done, failed, pending };
});

export function addRenameFiles(paths: string[]) {
  renameFiles.update((current) => {
    const existing = new Set(current.map((f) => f.path));
    const newFiles: OrganizeFile[] = paths
      .filter((p) => !existing.has(p))
      .map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        status: "pending" as const,
      }));
    return [...current, ...newFiles];
  });
}

export function removeRenameFile(path: string) {
  renameFiles.update((current) => current.filter((f) => f.path !== path));
}

export function clearRenameFiles() {
  renameFiles.set([]);
}

export function setOrganizeResult(result: { moved: number; skipped: number; errors: string[] } | null) {
  organizeResult.set(result);
}

export function clearOrganizeResult() {
  organizeResult.set(null);
}

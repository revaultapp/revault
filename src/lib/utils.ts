import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export async function browseOutputDir(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  return typeof dir === "string" ? dir : null;
}

export async function openOutputFolder(filePath: string): Promise<void> {
  await revealItemInDir(filePath);
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export async function runWithConcurrency<T>(
  items: T[],
  task: (item: T) => Promise<void>,
): Promise<void> {
  if (items.length === 0) return;
  const concurrency = Math.min(
    Math.max(2, (navigator.hardwareConcurrency || 4) - 2),
    items.length,
  );
  let nextIndex = 0;
  async function worker() {
    while (nextIndex < items.length) {
      await task(items[nextIndex++]);
    }
  }
  await new Promise<void>((r) => setTimeout(r, 0));
  await Promise.all(Array.from({ length: concurrency }, () => worker()));
}


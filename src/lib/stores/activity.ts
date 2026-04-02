import { writable } from "svelte/store";

export interface ActivityItem {
  id: string;
  type: "compress" | "convert" | "resize" | "analyze" | "organize" | "watermark" | "rename";
  fileCount: number;
  savedBytes: number;
  timestamp: number;
}

const STORAGE_KEY = "revault_activity";
const MAX_ITEMS = 3;

function loadActivity(): ActivityItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed.slice(0, MAX_ITEMS);
    }
  } catch {
    // ignore parse errors
  }
  return [];
}

function saveActivity(items: ActivityItem[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  } catch {
    // localStorage unavailable or quota exceeded
  }
}

function createActivity() {
  const initial = loadActivity();
  const { subscribe, update } = writable<ActivityItem[]>(initial);

  return {
    subscribe,
    add(entry: Omit<ActivityItem, "id" | "timestamp">) {
      if (entry.savedBytes < 0) return;
      update((items) => {
        const next: ActivityItem[] = [
          { ...entry, id: crypto.randomUUID(), timestamp: Date.now() },
          ...items,
        ].slice(0, MAX_ITEMS);
        saveActivity(next);
        return next;
      });
    },
  };
}

export const activity = createActivity();

export function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor((Date.now() - timestamp) / 1000);
  if (seconds < 60) return "Just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

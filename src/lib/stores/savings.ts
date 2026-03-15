import { writable } from "svelte/store";

const STORAGE_KEY = "revault_saved_bytes";

function createSavings() {
  const initial = parseInt(localStorage.getItem(STORAGE_KEY) ?? "0", 10) || 0;
  const { subscribe, update } = writable(initial);

  return {
    subscribe,
    add(bytes: number) {
      if (bytes <= 0) return;
      update((total) => {
        const next = total + bytes;
        localStorage.setItem(STORAGE_KEY, String(next));
        return next;
      });
    },
  };
}

export const savings = createSavings();

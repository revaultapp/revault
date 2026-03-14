import { writable } from "svelte/store";

export const activePage = writable("dashboard");
export const activeTool = writable<"compress" | "convert" | "resize">("compress");

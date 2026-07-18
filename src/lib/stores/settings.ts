import { persisted } from "$lib/utils";
import type { QualityPreset } from "./compress";
import type { VideoPreset, PrivacyMode } from "./video";

export const defaultOutputDir = persisted<string | null>("settings-default-output-dir", null);

// Global processing defaults. null = "remember last use" (the pre-existing
// per-tool behavior). These only seed each tool's store at init time — see
// persistedWithGlobalDefault in $lib/utils for the exact semantics.
export const defaultImagePreset = persisted<QualityPreset | null>("settings-default-image-preset", null);
export const defaultVideoPreset = persisted<VideoPreset | null>("settings-default-video-preset", null);
export const defaultVideoPrivacy = persisted<PrivacyMode | null>("settings-default-video-privacy", null);

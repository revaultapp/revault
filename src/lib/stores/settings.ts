import { persisted } from "$lib/utils";
import type { QualityPreset } from "./compress";
import type { VideoPreset, PrivacyMode } from "./video";

// Allowlist guards for persisted enum values. They live here (not in the tool
// stores) because compress/video already import settings at runtime — the
// reverse edge would create a cycle. Keep the compress/video imports above
// type-only for the same reason.
export const isQualityPreset = (v: unknown): v is QualityPreset =>
  v === "Smallest" || v === "Balanced" || v === "HighQuality";
export const isVideoPreset = (v: unknown): v is VideoPreset =>
  v === "Smallest" || v === "Balanced" || v === "HighQuality";
export const isPrivacyMode = (v: unknown): v is PrivacyMode =>
  v === "off" || v === "smart" || v === "gps_only" || v === "full";

const orNull = (guard: (v: unknown) => boolean) => (v: unknown) => v === null || guard(v);

export const defaultOutputDir = persisted<string | null>(
  "settings-default-output-dir",
  null,
  (v) => v === null || typeof v === "string",
);

// Global processing defaults. null = "remember last use" (the pre-existing
// per-tool behavior). Non-null values seed each tool's store at init AND
// propagate to it live on every later change — see persistedWithGlobalDefault
// in $lib/utils for the exact semantics.
export const defaultImagePreset = persisted<QualityPreset | null>(
  "settings-default-image-preset",
  null,
  orNull(isQualityPreset),
);
export const defaultVideoPreset = persisted<VideoPreset | null>(
  "settings-default-video-preset",
  null,
  orNull(isVideoPreset),
);
export const defaultVideoPrivacy = persisted<PrivacyMode | null>(
  "settings-default-video-privacy",
  null,
  orNull(isPrivacyMode),
);

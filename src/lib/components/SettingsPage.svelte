<script lang="ts">
  import { Sun, Moon, Monitor, FolderOpen, RotateCcw, ShieldCheck } from "lucide-svelte";
  import { theme } from "$lib/stores/theme";
  import { defaultOutputDir, defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } from "$lib/stores/settings";
  import type { QualityPreset } from "$lib/stores/compress";
  import type { VideoPreset, PrivacyMode } from "$lib/stores/video";
  import { browseOutputDir } from "$lib/utils";
  import { getLocale, setLocale, t } from "$lib/stores/locale.svelte";
  import type { Locale } from "$lib/i18n";
  import SegmentedControl from "./SegmentedControl.svelte";

  const REMEMBER = "remember";

  async function pickOutputDir() {
    const dir = await browseOutputDir();
    if (dir) defaultOutputDir.set(dir);
  }

  function resetOutputDir() {
    defaultOutputDir.set(null);
  }

  let languageSegments = $derived([
    { id: "en", label: t("settings.languageEnglish") },
    { id: "es", label: t("settings.languageSpanish") },
    { id: "fr", label: t("settings.languageFrench") },
    { id: "de", label: t("settings.languageGerman") },
    { id: "pt", label: t("settings.languagePortuguese") },
  ] as const);

  function selectLanguage(id: string) {
    setLocale(id as Locale);
  }

  let imagePresetSegments = $derived([
    { id: REMEMBER, label: t("settings.defaultRememberLast") },
    { id: "Smallest", label: t("common.qualitySmallest") },
    { id: "Balanced", label: t("common.qualityBalanced") },
    { id: "HighQuality", label: t("common.qualityHighQuality") },
  ] as const);

  let videoPresetSegments = $derived([
    { id: REMEMBER, label: t("settings.defaultRememberLast") },
    { id: "Smallest", label: t("video.presetSmallest") },
    { id: "Balanced", label: t("video.presetBalanced") },
    { id: "HighQuality", label: t("video.presetHighQuality") },
  ] as const);

  let videoPrivacySegments = $derived([
    { id: REMEMBER, label: t("settings.defaultRememberLast") },
    { id: "off", label: t("video.privacyOff") },
    { id: "smart", label: t("video.privacySmart") },
    { id: "gps_only", label: t("video.privacyGpsOnly") },
    { id: "full", label: t("video.privacyFull") },
  ] as const);

  function selectImagePreset(id: string) {
    defaultImagePreset.set(id === REMEMBER ? null : (id as QualityPreset));
  }

  function selectVideoPreset(id: string) {
    defaultVideoPreset.set(id === REMEMBER ? null : (id as VideoPreset));
  }

  function selectVideoPrivacy(id: string) {
    defaultVideoPrivacy.set(id === REMEMBER ? null : (id as PrivacyMode));
  }
</script>

<div class="content-wrap">
<div class="sections">
  <!-- General -->
  <section class="section">
    <div class="section-header">
      <h2>{t("settings.generalTitle")}</h2>
      <p>{t("settings.generalDesc")}</p>
    </div>
    <hr />
    <div class="row">
      <div class="label">
        <span class="name">{t("settings.themeLabel")}</span>
        <span class="desc">{t("settings.themeDesc")}</span>
      </div>
      <div class="segmented">
        <button class="seg" class:active={$theme === 'light'} onclick={() => theme.set('light')}>
          <Sun size={14} strokeWidth={2} />
          <span>{t("settings.themeLight")}</span>
        </button>
        <button class="seg" class:active={$theme === 'dark'} onclick={() => theme.set('dark')}>
          <Moon size={14} strokeWidth={2} />
          <span>{t("settings.themeDark")}</span>
        </button>
        <button class="seg" class:active={$theme === 'system'} onclick={() => theme.set('system')}>
          <Monitor size={14} strokeWidth={2} />
          <span>{t("settings.themeSystem")}</span>
        </button>
      </div>
    </div>
    <div class="row">
      <div class="label">
        <span class="name">{t("settings.language")}</span>
      </div>
      <SegmentedControl
        segments={languageSegments}
        selected={getLocale()}
        onselect={selectLanguage}
        label={t("settings.language")}
      />
    </div>
    <div class="row">
      <div class="label">
        <span class="name">{t("settings.defaultOutputFolderLabel")}</span>
        <span class="desc">{t("settings.defaultOutputFolderDesc")}</span>
      </div>
      <div class="output-controls">
        <button class="btn-ghost" onclick={pickOutputDir}>
          <FolderOpen size={14} strokeWidth={2} />
          <span class="output-name">{$defaultOutputDir?.split(/[\\/]/).pop() ?? t("common.sameAsInput")}</span>
        </button>
        {#if $defaultOutputDir}
          <button class="btn-ghost btn-icon" onclick={resetOutputDir} title={t("settings.resetOutputTitle")}>
            <RotateCcw size={14} strokeWidth={2} />
          </button>
        {/if}
      </div>
    </div>
  </section>

  <!-- Defaults -->
  <section class="section">
    <div class="section-header">
      <h2>{t("settings.defaultsTitle")}</h2>
      <p>{t("settings.defaultsDesc")}</p>
    </div>
    <hr />
    <div class="row wrap">
      <div class="label">
        <span class="name">{t("settings.defaultImagePresetLabel")}</span>
      </div>
      <SegmentedControl
        segments={imagePresetSegments}
        selected={$defaultImagePreset ?? REMEMBER}
        onselect={selectImagePreset}
        label={t("settings.defaultImagePresetLabel")}
      />
    </div>
    <div class="row wrap">
      <div class="label">
        <span class="name">{t("settings.defaultVideoPresetLabel")}</span>
      </div>
      <SegmentedControl
        segments={videoPresetSegments}
        selected={$defaultVideoPreset ?? REMEMBER}
        onselect={selectVideoPreset}
        label={t("settings.defaultVideoPresetLabel")}
      />
    </div>
    <div class="row wrap">
      <div class="label">
        <span class="name">{t("settings.defaultVideoPrivacyLabel")}</span>
      </div>
      <SegmentedControl
        segments={videoPrivacySegments}
        selected={$defaultVideoPrivacy ?? REMEMBER}
        onselect={selectVideoPrivacy}
        label={t("settings.defaultVideoPrivacyLabel")}
      />
    </div>
  </section>

  <!-- About -->
  <section class="section">
    <div class="section-header">
      <h2>{t("settings.aboutTitle")}</h2>
      <p>{t("settings.aboutDesc")}</p>
    </div>
    <hr />
    <div class="privacy-badge">
      <ShieldCheck size={18} strokeWidth={2} />
      <div class="privacy-badge-text">
        <span class="privacy-badge-title">{t("settings.privacyBadgeTitle")}</span>
        <span class="privacy-badge-desc">{t("settings.privacyBadgeDesc")}</span>
      </div>
    </div>
    <div class="row small">
      <span class="name">{t("settings.versionLabel")}</span>
      <span class="version-val">{__APP_VERSION__}</span>
    </div>
  </section>
</div>
</div>

<style>
  .content-wrap {
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 40px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .section-header h2 {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.025em;
    color: var(--text-primary);
  }

  .section-header p {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  hr {
    border: none;
    height: 1px;
    background: var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
  }

  .row.small {
    height: 36px;
  }

  .row.wrap {
    height: auto;
    min-height: 48px;
    flex-wrap: wrap;
    gap: 12px;
    padding: 4px 0;
  }

  .label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .segmented {
    display: flex;
    border-radius: var(--radius-sm);
    background: var(--navy-bg);
    overflow: hidden;
  }

  .seg {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: background-color 0.15s, border-color 0.15s;
  }

  .seg.active {
    background: var(--bg-card);
    color: var(--accent);
    font-weight: 600;
    box-shadow: 0 0 0 1px var(--border);
  }

  .version-val {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .privacy-badge {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-md);
    background: var(--accent-subtle);
    border: 1px solid var(--accent-glow);
    color: var(--accent-text);
  }

  .privacy-badge :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--accent);
  }

  .privacy-badge-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .privacy-badge-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--accent-text);
  }

  .privacy-badge-desc {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .output-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-ghost {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background-color 0.15s, border-color 0.15s;
  }

  .btn-ghost:hover {
    background: var(--navy-bg);
    border-color: var(--text-muted);
  }

  .btn-ghost.btn-icon {
    padding: 6px;
    color: var(--text-muted);
  }

  .output-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

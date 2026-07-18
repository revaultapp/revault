<script lang="ts">
  import { tick } from "svelte";
  import { Sun, Moon, Monitor, FolderOpen, RotateCcw, ShieldCheck } from "lucide-svelte";
  import { theme } from "$lib/stores/theme";
  import type { Theme } from "$lib/stores/theme";
  import { defaultOutputDir, defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } from "$lib/stores/settings";
  import type { QualityPreset } from "$lib/stores/compress";
  import type { VideoPreset, PrivacyMode } from "$lib/stores/video";
  import { browseOutputDir } from "$lib/utils";
  import { getLocale, setLocale, t } from "$lib/stores/locale.svelte";
  import type { Locale } from "$lib/i18n";
  import SegmentedControl from "./SegmentedControl.svelte";
  import Button from "./Button.svelte";

  const REMEMBER = "remember";

  // Screen-reader confirmation of applied changes. Composed exclusively from
  // existing locale strings ("<row label>: <value label>") — no new i18n keys.
  let announcement = $state("");
  function announce(rowLabel: string, valueLabel: string) {
    announcement = `${rowLabel}: ${valueLabel}`;
  }
  function labelOf(segments: readonly { id: string; label: string }[], id: string): string {
    return segments.find((s) => s.id === id)?.label ?? id;
  }

  let outputPickerEl: HTMLButtonElement | undefined = $state();
  let currentOutputName = $derived(
    $defaultOutputDir?.split(/[\\/]/).pop() ?? t("common.sameAsInput"),
  );

  async function pickOutputDir() {
    const dir = await browseOutputDir();
    if (dir) {
      defaultOutputDir.set(dir);
      announce(t("settings.defaultOutputFolderLabel"), dir.split(/[\\/]/).pop() ?? dir);
    }
  }

  function resetOutputDir() {
    defaultOutputDir.set(null);
    announce(t("settings.defaultOutputFolderLabel"), t("common.sameAsInput"));
    // The reset button unmounts with {#if $defaultOutputDir} while focused;
    // park focus on the sibling picker so it doesn't drop to <body>.
    tick().then(() => outputPickerEl?.focus());
  }

  let themeSegments = $derived([
    { id: "light", label: t("settings.themeLight"), icon: Sun },
    { id: "dark", label: t("settings.themeDark"), icon: Moon },
    { id: "system", label: t("settings.themeSystem"), icon: Monitor },
  ] as const);

  function selectTheme(id: string) {
    theme.set(id as Theme);
    announce(t("settings.themeLabel"), labelOf(themeSegments, id));
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
    // t() resolves in the just-chosen locale — announced in the new language.
    announce(t("settings.language"), labelOf(languageSegments, id));
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
    announce(t("settings.defaultImagePresetLabel"), labelOf(imagePresetSegments, id));
  }

  function selectVideoPreset(id: string) {
    defaultVideoPreset.set(id === REMEMBER ? null : (id as VideoPreset));
    announce(t("settings.defaultVideoPresetLabel"), labelOf(videoPresetSegments, id));
  }

  function selectVideoPrivacy(id: string) {
    defaultVideoPrivacy.set(id === REMEMBER ? null : (id as PrivacyMode));
    announce(t("settings.defaultVideoPrivacyLabel"), labelOf(videoPrivacySegments, id));
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
      <SegmentedControl
        segments={themeSegments}
        selected={$theme}
        onselect={selectTheme}
        label={t("settings.themeLabel")}
      />
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
        <Button
          variant="ghost"
          size="sm"
          bind:el={outputPickerEl}
          onclick={pickOutputDir}
          aria-label={`${t("settings.defaultOutputFolderLabel")}: ${currentOutputName}`}
        >
          <FolderOpen size={14} strokeWidth={2} />
          <span class="output-name">{currentOutputName}</span>
        </Button>
        {#if $defaultOutputDir}
          <Button
            variant="ghost"
            size="sm"
            class="btn-reset"
            onclick={resetOutputDir}
            title={t("settings.resetOutputTitle")}
            aria-label={t("settings.resetOutputTitle")}
          >
            <RotateCcw size={14} strokeWidth={2} />
          </Button>
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

<div class="visually-hidden" role="status" aria-live="polite">{announcement}</div>
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
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .section-header p {
    font-size: 13px;
    /* --chart-tick, not --text-muted: the legible small-text token
       (app.css documents --text-muted as failing AA at this size). */
    color: var(--chart-tick);
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
    color: var(--chart-tick);
  }

  .version-val {
    font-size: 13px;
    font-weight: 500;
    color: var(--chart-tick);
    font-variant-numeric: tabular-nums;
  }

  .privacy-badge {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px;
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

  /* Button's sm ghost lands at ~35px — enforce the 36px touch-target floor. */
  .output-controls :global(.btn-ghost) {
    min-height: 36px;
  }

  .output-controls :global(.btn-reset) {
    min-width: 36px;
    padding: 6px;
    justify-content: center;
  }

  .output-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>

<script lang="ts">
  import { tick } from "svelte";
  import type { ComponentType } from "svelte";
  import { theme } from "$lib/stores/theme";
  import type { Theme } from "$lib/stores/theme";
  import { defaultOutputDir, defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } from "$lib/stores/settings";
  import type { QualityPreset } from "$lib/stores/compress";
  import type { VideoPreset, PrivacyMode } from "$lib/stores/video";
  import { browseOutputDir } from "$lib/utils";
  import { getLocale, setLocale, t } from "$lib/stores/locale.svelte";
  import type { Locale } from "$lib/i18n";
  import {
    AppearanceDarkIcon,
    AppearanceLightIcon,
    AppearanceSystemIcon,
    ImageDefaultsIcon,
    LanguageIcon,
    OutputFolderIcon,
    ResetIcon,
    VideoDefaultsIcon,
  } from "$lib/components/settings-icons";
  import { PrivacyIcon } from "$lib/components/icons";
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
    { id: "light", label: t("settings.themeLight"), icon: AppearanceLightIcon as unknown as ComponentType },
    { id: "dark", label: t("settings.themeDark"), icon: AppearanceDarkIcon as unknown as ComponentType },
    { id: "system", label: t("settings.themeSystem"), icon: AppearanceSystemIcon as unknown as ComponentType },
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
    <section class="section">
      <div class="section-header">
        <h2>{t("settings.generalTitle")}</h2>
        <p>{t("settings.generalDesc")}</p>
      </div>
      <div class="workspace-grid">
        <article class="setting-card">
          <div class="setting-heading">
            <span class="setting-icon"><AppearanceSystemIcon /></span>
            <div class="label">
              <span class="name">{t("settings.themeLabel")}</span>
              <span class="desc">{t("settings.themeDesc")}</span>
            </div>
          </div>
          <div class="setting-control">
            <SegmentedControl
              segments={themeSegments}
              selected={$theme}
              onselect={selectTheme}
              label={t("settings.themeLabel")}
            />
          </div>
        </article>

        <article class="setting-card">
          <div class="setting-heading">
            <span class="setting-icon"><LanguageIcon /></span>
            <div class="label">
              <span class="name">{t("settings.language")}</span>
            </div>
          </div>
          <div class="setting-control">
            <SegmentedControl
              segments={languageSegments}
              selected={getLocale()}
              onselect={selectLanguage}
              label={t("settings.language")}
            />
          </div>
        </article>

        <article class="setting-card setting-card--wide">
          <div class="setting-heading">
            <span class="setting-icon"><OutputFolderIcon /></span>
            <div class="label">
              <span class="name">{t("settings.defaultOutputFolderLabel")}</span>
              <span class="desc">{t("settings.defaultOutputFolderDesc")}</span>
            </div>
          </div>
          <div class="output-controls">
            <Button
              variant="ghost"
              size="sm"
              bind:el={outputPickerEl}
              onclick={pickOutputDir}
              aria-label={`${t("settings.defaultOutputFolderLabel")}: ${currentOutputName}`}
            >
              <OutputFolderIcon size={14} strokeWidth={2} />
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
                <ResetIcon size={14} strokeWidth={2} />
              </Button>
            {/if}
          </div>
        </article>
      </div>
    </section>

    <section class="section">
      <div class="section-header">
        <h2>{t("settings.defaultsTitle")}</h2>
        <p>{t("settings.defaultsDesc")}</p>
      </div>
      <div class="defaults-surface">
        <article class="default-tile">
          <div class="setting-heading">
            <span class="setting-icon"><ImageDefaultsIcon /></span>
            <span class="name">{t("settings.defaultImagePresetLabel")}</span>
          </div>
          <div class="setting-control">
            <SegmentedControl
              segments={imagePresetSegments}
              selected={$defaultImagePreset ?? REMEMBER}
              onselect={selectImagePreset}
              label={t("settings.defaultImagePresetLabel")}
            />
          </div>
        </article>

        <article class="default-tile">
          <div class="setting-heading">
            <span class="setting-icon"><VideoDefaultsIcon /></span>
            <span class="name">{t("settings.defaultVideoPresetLabel")}</span>
          </div>
          <div class="setting-control">
            <SegmentedControl
              segments={videoPresetSegments}
              selected={$defaultVideoPreset ?? REMEMBER}
              onselect={selectVideoPreset}
              label={t("settings.defaultVideoPresetLabel")}
            />
          </div>
        </article>

        <article class="default-tile">
          <div class="setting-heading">
            <span class="setting-icon"><PrivacyIcon /></span>
            <span class="name">{t("settings.defaultVideoPrivacyLabel")}</span>
          </div>
          <div class="setting-control">
            <SegmentedControl
              segments={videoPrivacySegments}
              selected={$defaultVideoPrivacy ?? REMEMBER}
              onselect={selectVideoPrivacy}
              label={t("settings.defaultVideoPrivacyLabel")}
            />
          </div>
        </article>
      </div>
    </section>

    <section class="section">
      <div class="section-header">
        <h2>{t("settings.aboutTitle")}</h2>
        <p>{t("settings.aboutDesc")}</p>
      </div>
      <div class="about-surface">
        <div class="privacy-proof">
          <span class="setting-icon"><PrivacyIcon /></span>
          <div class="privacy-proof-text">
            <span class="privacy-proof-title">{t("settings.privacyBadgeTitle")}</span>
            <span class="privacy-proof-desc">{t("settings.privacyBadgeDesc")}</span>
          </div>
        </div>
        <div class="version-row">
          <span class="name">{t("settings.versionLabel")}</span>
          <span class="version-val">{__APP_VERSION__}</span>
        </div>
      </div>
    </section>
  </div>

  <div class="visually-hidden" role="status" aria-live="polite">{announcement}</div>
</div>

<style>
  .content-wrap {
    width: 100%;
    max-width: 920px;
    margin: 0 auto;
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 40px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-header h2 {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .section-header p {
    margin-top: 4px;
    font-size: 13px;
    /* --chart-tick, not --text-muted: the legible small-text token
       (app.css documents --text-muted as failing AA at this size). */
    color: var(--chart-tick);
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .setting-card,
  .default-tile,
  .about-surface {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
  }

  .setting-card,
  .default-tile {
    transition: transform var(--duration-normal) var(--ease-out);
  }

  .setting-card:hover,
  .setting-card:focus-within,
  .default-tile:hover,
  .default-tile:focus-within {
    --icon-duo: color-mix(in oklch, var(--accent) 18%, transparent);
    box-shadow: var(--shadow-md);
    transform: translateY(-2px);
  }

  .setting-card:focus-within,
  .default-tile:focus-within {
    border-color: var(--accent-text);
  }

  .setting-heading,
  .privacy-proof,
  .version-row,
  .output-controls {
    display: flex;
    align-items: center;
  }

  .setting-heading,
  .privacy-proof {
    gap: 12px;
  }

  .setting-heading {
    align-items: flex-start;
  }

  .setting-icon {
    display: block;
    flex: 0 0 auto;
    margin-top: 2px;
    color: var(--text-secondary);
  }

  .setting-card:hover .setting-icon,
  .setting-card:focus-within .setting-icon,
  .default-tile:hover .setting-icon,
  .default-tile:focus-within .setting-icon {
    color: var(--accent-text);
  }

  .label,
  .privacy-proof-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .name,
  .privacy-proof-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .desc,
  .privacy-proof-desc {
    font-size: 12px;
    color: var(--chart-tick);
  }

  .setting-control {
    min-width: 0;
  }

  .defaults-surface {
    display: grid;
    gap: 8px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--navy-bg);
  }

  .output-controls {
    flex-wrap: wrap;
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
    max-width: 240px;
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

  .about-surface {
    gap: 16px;
  }

  .privacy-proof {
    align-items: flex-start;
  }

  .privacy-proof .setting-icon {
    color: var(--accent-text);
  }

  .privacy-proof-title {
    font-weight: 600;
  }

  .version-row {
    justify-content: space-between;
    gap: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .version-val {
    flex: 0 0 auto;
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    color: var(--chart-tick);
  }

  @media (min-width: 760px) {
    .workspace-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .setting-card--wide {
      grid-column: 1 / -1;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }

    .setting-card--wide .output-controls {
      flex: 0 0 auto;
    }

    .defaults-surface {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .default-tile {
      padding: 16px;
    }
  }
</style>

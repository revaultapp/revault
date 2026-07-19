# ReVault — Next Steps

> Backlog de trabajo para retomar. Ordenado por secuencia lógica: primero lo que desbloquea, luego lo barato de alto impacto, luego lo que depende de un release real.
> Última actualización: 2026-07-18.

## Ya hecho (contexto, no rehacer)

**Sesión 2026-07-18 (censo de competidores + pulido UI):**
- **Censo de competidores famosos** con workflow deep-research (subagentes Sonnet) + re-verificación manual (API GitHub, curls) → `COMPETITOR_CENSUS_2026_07.md`. Conclusiones clave: (1) el hueco de ReVault CONFIRMADO — nadie famoso junta imagen+vídeo+PDF local-only cross-platform open-source; (2) **dedupe y privacidad no las tiene NADIE del censo** → decisión: subirlas de rango en README/About/landing (ver bloque nuevo en §1); (3) 60-68% del tráfico de los grandes es SEO orgánico → las landings por tarea de tryrevault.com son el canal; (4) alerta oficial FBI 2025-03-17 sobre conversores online falsos con malware = narrativa de seguridad verificable para el lanzamiento.
- Post-blitz cleanup merged (PR #88, `14a96b3`): CancelSlot, withListener/list helpers, savings de PDF Optimize en Dashboard, clippy estricto + render real pdfium en CI. 238 Rust + 180 frontend = 418 tests.
- Pulido UI directo a main: barras de modos centradas (PDF/Optimize), DropZone unificado 600×320 en todas las páginas (`e9240c7`, `f50c04e`); Dashboard: flechitas de tarjetas ahora alternan gráfico↔tabla accesible, causa raíz del overflow del donut arreglada (grid floors + overflow-x + clamp tooltip), 5 fixes de contraste AA, placeholders disabled con estilo muerto (sin commitear al cierre de sesión — ver estado del árbol).

**Sesión 2026-07-17 (tarde — Fase 1A + dashboard merged):**
- PR #85 merged (`91f2eb9`): **Images → PDF** (Fase 1A del plan §6). 208 Rust + 150 frontend en ese punto.
- PR #84 merged (`9b64e7a`): **dashboard analytics** (history store mensual + charts SVG a mano + fixes UI). Se resolvieron sus conflictos con #85 y se integró el modo Images→PDF en `history.recordSavings("pdf", ...)` para que cuente en el panel.
- Tests: **208 Rust + 162 frontend = 370**. Cero PRs abiertas, main en `9b64e7a`.
- ⚠️ **QA visual diferido** (ambas se mergearon sin pase manual, decisión del owner): en la próxima `pnpm tauri dev` revisar (a) Images→PDF: batch mixto HEIC+JPEG-rotado+PNG, Fit/Carta/márgenes, temas, locales; (b) Dashboard: empty states con perfil fresco, loop compress→charts, donut persiste tras restart, temas, locales, reduced motion. Follow-ups declarados del #84: podar ~31 claves i18n `dashboard.*` huérfanas, headers de tablas ocultas solo-inglés, botones range/share/export placeholder.
- Cuenta gh: el CLI tenía activa `tessera-dev` (sin permisos) — cambiada a `miguelabdonsh` con `gh auth switch`.

**Sesión 2026-07-17 (mañana — estudio de demanda + verificación adversarial):**
- Estudio "¿qué es el 85% de uso real?" con 6 subagentes (búsqueda, comunidad ConvertX/Stirling/VERT, comercial, usuario casual, PDF/docs, cola del 15%) → `USAGE_DEMAND_STUDY_2026_07.md`.
- Verificación adversarial con 5 subagentes de todas las afirmaciones load-bearing (fuentes primarias, HTML crudo) → §7 del mismo doc. El enfoque sobrevivió; estimaciones corregidas (jpg→pdf 2-3d→4-6d; JXL menos inminente; mp3-keyword degradado a conjetura).
- Conclusión: ReVault ya cubre casi todo el 85%. Solo 3 huecos reales → plan por fases en **sección 6** de este doc. **Office↔PDF descartado formalmente** (movido a `.claude/memory/house/deferred.md` con watch-list).
- Dominios: **DECIDIDO — el dominio será `tryrevault.com`**; se compra justo antes del deploy, con el producto terminado (no está registrado aún — re-verificar disponibilidad al comprarlo). El nombre se mantiene "ReVault". Ojo colisión: revault.org = protocolo Bitcoin "Revault".

**Sesión 2026-07-11 (pulido UI/UX + CI hardening):**
- Pase de pulido UI/UX de toda la app: 8 commits push directo a `main` (`7699e0c..f6de104`). Tokens `--cat-*` + state-layers, `ToolShell` flip/out, helper `motion.ts` `animatedNumber`, chips de metadata en Privacy, panel Estimated animado en Compress/Video, Dashboard stats animadas. Frontend tests 139→142.
- Fix CI real: bug de YAML en `ci.yml` (`::` sin comillas) que rompía TODO CI desde el #81 → arreglado (`b586055`).
- PR #83 merged — a11y: tokens `--cat-*-text` (contraste iconos chip Privacy a AA en claro), `aria-pressed`/`aria-label` en pills y botones icon-only.
- PR #82 merged — CI hardening: MSRV `1.88` declarado + verificado en CI (valor determinista del árbol de deps, no adivinado), `.github/PULL_REQUEST_TEMPLATE.md` con checklist seguridad/arquitectura, branch protection ahora exige status checks `quality`+`platform-test (macos/windows)`.
- `main` en `fa5745a`, CI verde, 337 tests, sin deuda P0/P1/P2. **Nota:** desde #82 el merge exige review CI verde; en repo de un maintainer solo se mergea con `gh pr merge --admin`.

**Sesión 2026-07-07:**
- PR #81 merged — `cargo-deny` (licencias/sources) + check de pureza `core/` (grep `tauri::`) en CI. Detalle y lo que queda de este frente → **`CI_HARNESS_PLAYBOOK.md`** (branch protection sin required status checks es el único gap Tier 1 real).

**Sesión 2026-07-06 (presentación + web):**
- PR #79 merged — Tier 0: README sin claims falsos (`organization`, `exact-size targeting`), hero + 3 badges + bullets Privacy/Offline + texto Gatekeeper mejorado, `SECURITY.md` enlazado, `contact_link` de seguridad en el issue template, `exact-size targeting` fuera del CHANGELOG.
- PR #80 merged — banner hero del README (`.github/assets/banner.png`, generado con Satori+resvg).
- Fix local `.claude/hooks/stop-check.sh` — gate por ficheros sucios (no corre `pnpm check` en turnos sin cambios de frontend).
- Web landing (Astro + Tailwind) migrada a repo aparte `miguelabdonsh/web-revaultapp` (scaffold pusheado, `docs/PLAN.md` ahí es la fuente viva). Pendiente real: `AGENTS.md` propio de la web, deploy Cloudflare Pages, assets reales.

**Sesión previa:**
- Auditoría completa de `.claude/` (agentes, hooks, skills, memoria) — cerrada.
- PR #78 merged: fix del hook `compress_build_output`, `quality-check` por-archivo, `settings.local.json` colapsado (155→31), skills/agentes sincronizados con el código real.
- Sesión "truth-sync" aplicada: memorias purgadas de deuda-fantasma, contadores de test de-duplicados a `deferred.md`.
- Worktrees y ramas temporales eliminados. `main` limpio, CI verde, 334 tests, sin deuda P0/P1/P2.

---

## 0. Gates de release — camino crítico (bloquean un lanzamiento público real)

Estos NO son pulido; sin ellos el release convierte mal o no instala. Requieren decisión/acción humana tuya.

- [ ] **Firma de código macOS** — cuenta Apple Developer ($99/año) + configurar secrets en GitHub (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `KEYCHAIN_PASSWORD`). El `release.yml` ya tiene el flujo gateado tras `secrets.APPLE_CERTIFICATE != ''`; sin secrets, el build sale **sin firmar** → Gatekeeper bloquea.
- [ ] **Firma de código Windows** — SignPath (o cert propio) pendiente. Sin firma → SmartScreen advierte.
- [ ] **Smoke test real en Windows** — nunca se ha ejecutado la app de verdad en Windows (el CI solo compila). Verificar: arranca la UI, HEIC decodifica, el sidecar de ffmpeg lanza, drag&drop funciona.
- [ ] **Smoke test real en Linux** — mismo caso (.AppImage/.deb).
- [ ] **Promover CHANGELOG** — `## [Unreleased]` → `## [0.1.0] - 2026-XX-XX` (CHANGELOG.md:8).

> Decisión pendiente: ¿"v0.1.0" es tag/beta en Mac para probadores (listo casi ya), o descarga pública sin fricción (requiere los gates de arriba)?

---

## 1. Presentación Tier 0 — barato, alto impacto, cero dependencias (se puede hacer YA)

Todo texto/config. Claude puede aplicar casi todo en una rama para revisar el diff.

> ✅ **HECHO vía PR #79** (banner en #80): bug del README, hero, bullets Privacy/Offline, texto Gatekeeper, 3 badges, enlace a `SECURITY.md`, `config.yml` de seguridad. **Único pendiente de esta sección: GitHub About** (abajo).

### ⭐ Reposicionar dedupe + privacidad como diferenciadores hero (decisión 2026-07-18, fuente: `COMPETITOR_CENSUS_2026_07.md` §7-8)

El censo confirmó que **ningún competidor famoso tiene dedupe** y **nadie trata el strip de metadatos como feature central** — son nuestras dos únicas features sin competencia, y hoy están enterradas como bullets del montón. Es trabajo de PRESENTACIÓN, no de código:

- [ ] **Tagline/hero del README**: que nombre lo único, no solo lo común. Candidata actualizada:
  `Compress, deduplicate, and strip private metadata from your photos, videos and PDFs — 100% offline, nothing ever leaves your machine.`
- [ ] **Orden de features en el README**: Duplicates y Privacy suben a las posiciones 2-3 (tras Compress), cada una con screenshot propio — no al final de la lista.
- [ ] **GitHub About**: la description ya las nombra (`...privacy, and duplicate cleanup`) — mantenerlas al frente cuando se aplique (§ GitHub About abajo).
- [ ] **Landing (tryrevault.com / repo web-revaultapp)**: sección propia para cada una + la narrativa de seguridad con fuentes (alerta FBI 2025-03-17 + Malwarebytes/CloudSEK sobre conversores online falsos con malware): *"tus archivos nunca salen de tu máquina"* como seguridad verificable, no solo privacidad. Detalle y citas en `COMPETITOR_CENSUS_2026_07.md` §8.
- [ ] **Copy de social/Show HN**: liderar con "finds duplicates & strips GPS/EXIF locally" — es el ángulo que r/privacy y Privacy Guides van a amplificar.

- [ ] **Bug del README** — la 1ª línea vende *"organization"* como feature, pero está **eliminada del scope** (AGENTS.md). Quitarlo (dato falso en la primera pantalla).
- [ ] **Reescribir el hero** — orden correcto: logo centrado → tagline en negrita (1 frase) → demo visual → *después* features (hoy van al revés). Tagline: ver bloque ⭐ arriba (la candidata vieja no nombraba dedupe/privacy).
- [ ] **Bullets Privacy & Offline** — 4 bullets verificables: 100% local · sin cuenta/servidor · sin telemetría · código auditable en `src-tauri/src/core/`. (NO copiar E2E/auditorías externas de Spacedrive — ReVault no las tiene.)
- [ ] **Mejorar texto Gatekeeper/SmartScreen** — ya existe; añadir el *"por qué"* (app sin firmar = normal, no red flag) al estilo `nrjais/sanchaar` + `Juniper/jccm`. Baja el abandono en el primer arranque.
- [ ] **3 badges** (no más, mismo estilo `flat`): CI · License · Platform. Placeholders `OWNER/REPO`:
  - `github/actions/workflow/status/OWNER/REPO/ci.yml?branch=main`
  - `github/license/OWNER/REPO`
  - static `platform-macOS | Windows | Linux-blue`
- [ ] **Enlazar SECURITY.md / CONTRIBUTING.md** en el README (los archivos existen, no se citan).
- [ ] **GitHub About** (hoy vacío) vía `gh repo edit`:
  - description: `Offline-first desktop app for image & video compression, privacy, and duplicate cleanup.`
  - homepage: link a `/releases` (hasta tener landing).
  - 12 topics: `tauri svelte rust desktop-app cross-platform image-compression image-optimization video-compression privacy offline-first exif pdf-tools`
  - Sube community score 87% → ~100% y arregla descubribilidad.
- [ ] **`.github/ISSUE_TEMPLATE/config.yml`** — añadir `contact_link` de seguridad (evita reportes de vulns en issues públicos).

---

## 2. Presentación Tier 1 — assets visuales (necesita `pnpm tauri dev` + Kap)

El hueco más grande: el README no tiene **ni una imagen** y ReVault es una app visual. Patrón de referencia: `kimlimjustin/xplorer` (Tauri 2, mismo stack), `logseq`, `pot-app/pot-desktop`.

> ✅ **Banner hero del README hecho** (#80). El resto de assets visuales (GIF, screenshots por feature, social preview) los captura el usuario con `pnpm tauri dev` + Kap; la landing web ya migró a `miguelabdonsh/web-revaultapp` (ver `docs/PLAN.md` de ese repo).

- [ ] **GIF hero** corto arriba: drag&drop → compress → resultado (barra de ahorro verde `#10D87A`). Herramienta: **Kap** (no VHS/asciinema — son de terminal).
- [ ] **1 screenshot por feature** junto a cada bullet (Optimize/Duplicates/Privacy/Video/PDF), no galería genérica. Guardar en `.github/assets/`.
- [ ] **Social preview 1280×640** (fondo `#0c0f0e`, logo, Plus Jakarta Sans, detalle verde) → Settings → Social preview. Sin ella, el link en Show HN/Twitter sale solo con el avatar.
- [ ] **Diagrama Mermaid** (core/ vs commands/ vs frontend) — nativo en GitHub, cero binario que mantener.

---

## 3. Presentación Tier 2 — el release (desbloquea lo demás)

- [ ] **`tag + push v0.1.0`** → dispara el `release.yml` existente (matriz de 4 builds: macOS arm64/x64, Windows, Linux). Reemplaza los "releases" actuales (que son binarios de ffmpeg/gifski, no la app).
- [ ] **Tabla de descargas** en el README con links reales por plataforma (patrón `lossless-cut` / `holochain/launcher`). Bloqueado hasta tener la release publicada.
- [ ] **Badges de release** — añadir `github/v/release` y `github/downloads/.../total` (solo tienen sentido cuando el número no sea 0).
- [ ] **Release notes** con bloque de highlights por feature (borrador ya redactado en la sesión de research).

---

## 4. Distribución Tier 3 — post-launch, sostenido

- [ ] **Día 0**: Show HN (enlaza al **repo**, no a landing; primer comentario preparado, sin superlativos) + r/opensource + r/privacy (escalonados) + PR a `tauri-apps/awesome-tauri` + submit a madewithtauri.com.
- [ ] **Sostenido**: AlternativeTo.net (alternativa a TinyPNG/CloudConvert/Squoosh/HandBrake) + Privacy Guides (categoría Photo Organization, vía foro) + awesome-mac + Lobsters (requiere invitación).
- [ ] **Diferido a v1.0**: Product Hunt (bala única, reservar para un hito con audiencia previa).
- Descartados con criterio (no forzar): r/selfhosted, awesome-selfhosted — scope = servicios de red, no apps de escritorio.

---

## 5. Tooling `.claude/` — menor, independiente, baja prioridad

- [x] **`stop-check.sh`** — HECHO: comentario corregido (dispara cada turno, no al cerrar sesión) + **gate por ficheros sucios** (solo corre `pnpm check` si hay `.svelte/.ts/.js` modificados). Fix local (`.claude/` gitignored).
- [ ] **Auditar salida de la sesión "truth-sync"** a fondo (memorias/agentes rust+devops) — no se revisó porque es gitignored y no fue parte de #78. Opcional.

---

## 6. Producto: cerrar el 85% real — plan verificado (2026-07-17)

> Fuente y justificación completa: `USAGE_DEMAND_STUDY_2026_07.md` (§6 recomendación, §7 verificación). Orden = ROI. Total estimado Fases 1A+1B+2: ~2.5-3 semanas.

### Fase 1A — jpg→pdf (scan-to-PDF multipágina) · ✅ MERGEADA — PR #85 → main `91f2eb9` (2026-07-17)

> Completa: core + command + store + UI + i18n ×3 + 21 tests nuevos (208 Rust + 150 frontend = 358). CI verde, squash-merged. Nota: se mergeó sin el QA manual del checklist de la PR — hacer ese pase visual (batch mixto HEIC+JPEG-rotado+PNG, Fit/Carta/márgenes, temas, locales) en la próxima sesión de `pnpm tauri dev` como verificación diferida.
- [ ] Activar feature `embed_image` de lopdf en `Cargo.toml` (no está en los defaults).
- [ ] `core/pdf.rs`: `images_to_pdf(paths, opts) -> PdfResult` — construir Pages tree/XObject/content stream a mano (no hay prior art en el repo: el código PDF actual solo reescribe streams existentes).
- [ ] **Checklist de gotchas verificados** (el estimado vive aquí):
  - Orientación EXIF: el embed DCTDecode directo la ignora → leer `orientation()` + `apply_orientation()` (image ≥0.25.4) y re-encodar rotado vía mozjpeg, o matriz `cm` de rotación.
  - PNG con alpha: lopdf descarta el alpha en silencio → SMask (segundo XObject gris) a mano.
  - CMYK: sin arm en `ColorType`; `compress_pdf_images` lo esquiva — aquí convertir a RGB o rechazar con mensaje claro.
  - DPI: px→pt (el test de lopdf usa píxeles crudos como MediaBox = página de 42×56"). Opciones de página Fit/A4/Carta + márgenes (paridad iLovePDF/Smallpdf).
  - HEIC: llega como píxeles decodificados (no bytes JPEG) → vía FlateDecode o re-encode mozjpeg.
- [ ] `commands/pdf.rs`: `process_images_to_pdf` (patrón thin: spawn_blocking + un map_err, `core::paths`, no-clobber).
- [ ] UI en `PdfPage.svelte` (modo nuevo) + i18n en/es/fr + tests Rust y de store.

### Fase 1B — MP4→MP3 (extracción de audio) · ✅ MERGEADA — PR #86 → main `548270a` (2026-07-17)

> Completa: probe de audio + plan puro testeable + extract con progreso/cancel/reaping, modo Auto (AAC→m4a copy byte-exacto, MD5 verificado contra ffmpeg real) / MP3 128-192-320, carátula attached_pic, flag de cancel propio, 4º modo en VideoPage, i18n ×3. Tests: **223 Rust + 166 frontend = 389**. CI verde, squash-merged. QA visual diferido (checklist en la PR). **Con esto, Fases 1A+1B del plan del 85% completas — solo queda Fase 2 (pdf→jpg) como decisión de dependencia aparte.**
- [ ] `core/video.rs`: `extract_audio(path, opts)`. ffprobe primero: AAC → `-c:a copy` a `.m4a`; resto → `-c:a libmp3lame` (encoder **verificado en los 4 builds pineados**, incl. arm64 local).
- [ ] Args robustos: `-map 0:a:0` explícito (auto-select elige por nº de canales), `-map_metadata 0`, rama para carátula `attached_pic` (un `-vn` a secas la pierde).
- [ ] Selector de bitrate (128/192/320) — un botón fijo queda por detrás de FreeConvert/CloudConvert.
- [ ] Cancelación reutilizando el patrón `cancel_video_compress`. UI: modo en `VideoPage.svelte` + i18n + tests.
- [ ] Compliance: binario Linux (johnvansickle) es **GPLv3** → source-offer en THIRD_PARTY_LICENSES (mismo trato que gifski).

### Fase 2 — pdf→jpg (rasterizado) · ~1 semana · única decisión de dependencia
- [ ] `pdfium-render` + binarios de `bblanchon/pdfium-binaries`: 3.2-3.6MB/plataforma, releases **inmutables** → pin SHA-256 + descarga on-first-use (patrón FFmpeg/gifski). **Evitar assets `pdfium-v8-*`** (11-27MB, V8 innecesario).
- [ ] **Diseño obligatorio: un solo hilo dueño de `Pdfium`** (actor + canal). NO fiarse del feature `thread_safe` (bug de soundness #262 abierto en 0.9.3; reevaluar cuando 0.9.4 publique el fix).
- [ ] `core/pdf.rs`: `pdf_to_images(path, dpi, format)` — selector DPI 150/300, manejo de PDFs cifrados/malformados (error claro, no blank).
- [ ] Compliance: bundle de ~10 licencias third-party del binario pdfium en THIRD_PARTY_LICENSES.
- [ ] Command + UI en PdfPage + i18n + tests con PDF fixture.

### Fase 2 — pdf→jpg (rasterizado) · ✅ IMPLEMENTADA — PR #87 (2026-07-18)

> Completa en `feat/pdf-to-images`: `pdfium-render` + dylib bundleada (fetch con SHA pin, firmada dentro del .app, verificado), actor de hilo único que neutraliza el bug #262, command con progreso/cancel, 5º modo en PdfPage, i18n ×3. Revisión adversarial de 5 dimensiones aplicada (7 fixes incl. 1 HIGH de resiliencia a panic y 1 P0 de compilación por `bundle.resources` en clon fresco → `.gitkeep` commiteado). Tests: **233 Rust + 173 frontend = 406**; render real verificado E2E + `pnpm tauri build` confirma bundle firmado. **Pendiente: CI verde + QA manual del owner** (checklist en la PR). **Con esto el plan del 85% (§6) queda completo salvo Fase 3, que es descarte.**

### Fase 3 — Office↔PDF: NO construir (decisión formal, no hueco)
Descartado con evidencia reforzada (x2t=AGPL; LibreOffice 293-345MB y headless crashea en macOS; competidores CON LibreOffice tienen pdf→docx roto en producción). Registrado en `deferred.md` + watch-list (dxpdf/office2pdf en 6-12 meses; JXL encode cuando Chrome default-on).

> Transversal (señal más fuerte del estudio de comunidad): infra/UX/batch/fiabilidad out-poll a cualquier formato nuevo. Antes de Fase 2, considerar si batch robusto o pulido de fiabilidad mueve más la aguja.

## Recordatorio

La presentación (Tiers 1-4) **amplifica** un release sólido; no lo sustituye. El camino crítico real sigue siendo la **sección 0** (firma + smoke test cross-platform). El README precioso con una app que no arranca en Windows convierte peor que un README feo con un binario que funciona.

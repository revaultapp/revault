# Estudio de demanda real de conversión/optimización de archivos — ¿qué es el 85% vs. el 15%?

**Fecha:** 2026-07-17
**Estado:** Investigación completa, sin implementación. Documento de referencia, no roadmap comprometido.
**Verificación adversarial:** 2026-07-17 — 5 subagentes intentaron refutar cada afirmación load-bearing contra fuentes primarias. El enfoque sobrevivió; correcciones aplicadas inline (marcadas ✎) y resumidas en §7.
**Método:** 6 subagentes en paralelo (Sonnet), cada uno sobre un ángulo distinto: (1) ranking cuantitativo de búsqueda, (2) señales de comunidad ConvertX/Stirling/VERT, (3) preferencia revelada de 8 conversores comerciales, (4) comportamiento del usuario casual, (5) demanda documento/PDF, (6) cuantificación de la cola del 15%.
**Disparador:** Decidir si vale la pena construir más allá de lo que ReVault ya cubre, o si el feature-set actual ya cubre el ~85% del uso real.

> **Advertencia metodológica (aplica a todo el documento):** ninguna de estas apps publica telemetría de uso por conversión. Un estudio honesto triangula señales de *preferencia revelada*: volumen de búsqueda, prominencia en homepage/nav (espacio prime = demanda), frecuencia de feature-requests/bugs en GitHub, listas de "populares" que sí publican algunos vendors. Cada dato va etiquetado **DATO** (cifra de fuente primaria) / **PROXY** (señal indirecta real) / **CONJETURA** (inferencia). La forma del ranking es fiable; los números absolutos son magnitud aproximada.

---

## 1. Resumen ejecutivo

- **ReVault ya cubre la mayor parte del 85%.** Los 6 ángulos convergen: comprimir (imagen+PDF), HEIC→JPG, JPG/PNG/WebP entre sí, resize, dedupe, strip de privacidad y compresión de video son exactamente el núcleo de alta demanda — y ReVault ya los tiene. El instinto de "no hacer más de la cuenta" es correcto.
- **El 15% de cola está confirmado como seguro de cortar** por múltiples fuentes independientes: 3D (salvo STL), ebooks (MOBI oficialmente muerto, Calibre domina), email (.msg = enterprise/legal), datos (json/yaml = developers), CAD/vector-pro, fuentes, contactos. En los trackers de ConvertX/VERT son asks de un solo usuario con 0-3 reacciones; ningún conversor comercial los promociona en su home pese a soportarlos.
- **Solo hay 3 huecos de nivel-85% que ReVault NO cubre**, y su ROI es muy distinto:
  1. **PDF↔Word / Office↔PDF** — la conversión *individual* de más demanda de toda la categoría (~220K/mo "pdf to word" en Ahrefs; #1 en Smallpdf; mayor volumen de bugs en ConvertX). **Pero es una trampa de ROI:** la herramienta barata (Pandoc+Tectonic) NO puede entregarlo a fidelidad aceptable, y la que sí (LibreOffice headless) pesa 300-400MB y es frágil. Alta demanda, coste alto para hacerlo *bien*.
  2. **Imagen↔PDF (jpg→pdf, pdf→jpg)** — alta demanda (scan-to-PDF móvil), y **barato**: no necesita LibreOffice, es rasterizar/embeber, encaja en el módulo PDF que ya existe. **La mejor oportunidad: alta demanda, bajo coste, on-brand.**
  3. **Extracción de audio (MP4→MP3)** — demanda alta (keyword citado ~246K/mo; ✎ magnitud no verificable en fuente primaria, tratar como conjetura direccional). FFmpeg ya está cableado y `libmp3lame` está **verificado en los 4 builds pineados**; coste real ~2-3 días (detección de códec vía ffprobe, `-map` explícito, carátula, selector de bitrate) — barato, no "casi gratis". Oportunidad secundaria.
- **Redirección clave del roadmap previo:** el estudio de ConvertX recomendaba un paquete Rust puro (SVG, spreadsheets, EPUB, 3D-lectura, ~3 semanas) + evaluar Pandoc+Tectonic. Este estudio de demanda dice que **casi todo ese paquete Rust apunta a la cola del 15%**, y que Pandoc+Tectonic apunta a una necesidad real (Office) que **no puede cumplir bien**. La demanda no respalda ninguno de los dos como prioridad.

---

## 2. La cabeza de la demanda (el ~85%) — convergencia de los 6 ángulos

Ordenado por fuerza de señal combinada. La columna "ReVault" indica cobertura actual.

| # | Necesidad | Señal | ReVault |
|---|---|---|---|
| 1 | **Comprimir imagen** | TinyPNG es líder de categoría *solo* con esto (~117K usuarios/mes, API en 50K+ empresas, DATO). App iOS "Image Compress & Resize" con **2M+ usuarios** bundlea compress+resize+HEIC local — casi un clon del core de ReVault, validado (DATO). Todo conversor tiene bucket "Compress" en nav. | ✅ Core |
| 2 | **PDF: comprimir / merge / split** | Compress+Merge en el top-3 de nav de todo player PDF (Smallpdf, iLovePDF, Adobe). Smallpdf: 3 tools = **69% de todo el uso** (Compress 34% + Sign 19% + PDF→Word 16%) (DATO). | ✅ Core |
| 3 | **PDF↔Word / Office↔PDF** | La conversión individual de más demanda de todo el estudio: ~220K/mo "pdf to word", KD 81 (Ahrefs, DATO); página PDF→Word de Adobe ~385K visitas/mes (DATO); #1 en el grid de Smallpdf; **mayor volumen de bugs** en ConvertX (gente usándolo de verdad). | ❌ **Hueco (trampa de ROI, ver §4)** |
| 4 | **Imagen↔PDF (jpg→pdf, pdf→jpg)** | La conversión cross-domain más grande; scan-to-PDF móvil. Top-3 en Smallpdf, presente en todo player. | ❌ **Hueco (oportunidad barata, ver §4)** |
| 5 | **HEIC→JPG** | "My most common web conversion. HEIC to JPG" — corroborado por 3 comentaristas en HN. Landing dedicada en TODO conversor + apps standalone. **Sin cifra numérica en ninguna parte** (mayor punto ciego), pero ubicuidad = demanda real. | ✅ Core |
| 6 | **JPG/PNG/WebP entre sí** | WebP→PNG es el #1 keyword de CloudConvert (~165K/mo, PROXY). PNG↔JPG citado como "la conversión de imagen más pedida" (pero sin fuente primaria, CONJETURA). Backends de imagen son las tablas más grandes de ConvertX. | ✅ Core |
| 7 | **Video: comprimir para mensajería, MOV→MP4** | Límite duro WhatsApp 16MB (un clip 4K de 1min ≈ 400MB); iPhone graba HEVC-in-MOV que Windows no lee. Ecosistema denso de tools. | ✅ Core |
| 8 | **Extracción de audio (MP4→MP3)** | "mp3 converter" citado como keyword #1 de CloudConvert (~246K/mo) — ✎ **no verificable en fuente primaria** (Semrush gated), tratar como CONJETURA de magnitud. La demanda de categoría sigue siendo real (tool dedicado en todo conversor). | ❌ **Hueco (barato: FFmpeg cableado + libmp3lame verificado en los 4 builds, ~2-3 días)** |
| 9 | **Duplicados / liberar espacio** | Apple lo metió nativo en Fotos (iOS 16/18.1) — señal de que consideró el dolor masivo (DATO). ReVault cubre el hueco desktop/cross-device que la de Apple (solo iOS) no. | ✅ Core |
| 10 | **Strip de privacidad EXIF/GPS** | Explainers mainstream (Norton, Proton) para público general; tema en alza. Menor frecuencia que comprimir. | ✅ Core |
| 11 | **Resize (email/web/perfil)** | Specs por plataforma buscadas (LinkedIn 400×400, etc.). Cubierto; sin picker de presets (nicety, no hueco). | ✅ Core |
| 12 | **GIF** | Real pero menor; incumbentes fuertes (Giphy/Canva). Solo señal cualitativa. | ✅ Tiene |

**Forma del power-law (estimación, no medida):** top ~10 conversiones capturan ~60-75% de la demanda "convert X to Y"; top ~20 ~80-90%. Base: Smallpdf e iLovePDF (competidores optimizando por separado) convergen en el **mismo top-5 exacto** mientras cada uno arrastra 160K+ keywords de cola; el 94.74% de todos los keywords de Google reciben ≤10 búsquedas/mes (DATO adyacente, ranktracker).

---

## 3. La cola (el ~15%) — confirmada como seguro de cortar

Cada uno aparece en los trackers de ConvertX/VERT como asks de un solo usuario (0-3 reacciones, sin discusión) y **ningún conversor comercial lo promociona en su home** pese a soportarlo.

| Categoría | Veredicto | Evidencia clave |
|---|---|---|
| **3D (obj/stl/gltf/fbx/+70)** | Cortar, salvo matiz STL | STL tiene población hobbyista real (Thingiverse/MakerWorld, millones), **pero casi nadie *convierte* — el slicer come STL directo**. Los otros 70+ formatos: cero señal consumer. |
| **Ebooks (epub/mobi/azw3)** | Cortar | **MOBI oficialmente muerto** (Amazon dejó de aceptarlo 2021, Send-to-Kindle 2023, DATO). Calibre domina, público self-publisher. |
| **Email (.msg/.eml/.pst) + contactos** | Cortar (público equivocado) | Todo el tooling es eDiscovery/forense/legal/migración enterprise. Cero hilos consumer. VCF real pero one-off trivial. |
| **Datos (json/yaml/toml/xml/csv)** | Cortar (público equivocado) | Explícitamente para developers/DevOps. Demanda plausiblemente grande pero **cero solape** con público de un optimizador de fotos. |
| **Vector/diseño (eps/ai/cdr/dxf)** | Cortar (salvo SVG leve) | SVG tiene demanda web durable; EPS legacy (Adobe lo dice); AI/CDR/DXF = pro-only, requieren software propietario para originarse. |
| **Fuentes (ttf/otf), iconos (icns)** | Cortar | Asks aislados repetidos por individuos distintos, sin upvotes. |
| **JPEG XL encode** | Cortar; **reevaluar cuando Chrome lo active por defecto** | ✎ Corregido tras verificación: Safari = soporte **parcial** desde iOS 17 (sin animación ni progresivo); Firefox 152 solo **compila** el decoder pero el pref va **desactivado** en release (on solo en Nightly); Chrome 145 tras flag, y la fecha de default-on no tiene fuente primaria. Señal direccional real, pero menos inminente de lo que decía el estudio inicial. |
| **RAW (dng/cr2/nef)** | Nicho durable, no mainstream | DNG ahora estándar ISO (2026, DATO). Vertical de fotógrafos, no consumer masivo. |

---

## 4. El hueco Office↔PDF — por qué es una trampa de ROI (hallazgo load-bearing)

El agente de documento/PDF encontró lo decisivo para la decisión de build:

- **La demanda de PDF↔Word quiere que el documento *siga pareciéndose al original*** (contratos, CVs, informes con layout fijo), no solo "las palabras correctas en formato editable".
- **Pandoc+Tectonic re-maqueta vía LaTeX** — sus fallos documentados (issue tracker propio): tablas con celdas fusionadas pierden filas/columnas, formas vectoriales de Word **no convierten en absoluto**, referencias cruzadas desaparecen, tablas sin bordes por defecto. Es una herramienta de *re-tipografiado*, no de *preservación de layout*.
- Las conversiones en las que Pandoc **sí** brilla (docx↔md↔html↔epub) están en la cola: iLovePDF pone "PDF to Markdown" **último de 31 tools** en su propia web.
- **LibreOffice headless** es arquitectónicamente lo que usan los incumbentes para Office↔PDF, pero pesa 300-400MB comprimido, cold start 10-12s, y aun así no clona el render de Word. Ya descartado en el estudio de ConvertX por presupuesto.

**Conclusión:** el bundle Pandoc+Tectonic (~230MB en Windows) compra una capacidad (round-trip markdown/epub) que la demanda **no** respalda como prioritaria, mientras que la capacidad que la demanda **sí** respalda (PDF↔Word con fidelidad) **no es lo que ese toolchain sabe entregar**. Es la peor combinación posible de ROI.

---

## 5. Dos hallazgos transversales (no-formato)

1. **Infra/UX out-poll a todo formato.** En el tracker de ConvertX, los issues de más engagement no son de formatos: SSO (38 reacciones), REST API (20), settings/customización (14), bugs de arranque Docker (46 comentarios). Para ReVault, análogo: **pulido, batch, fiabilidad y descubribilidad importan más que un formato nuevo.**
2. **PDF forms/edición es demanda real** — el issue #1 histórico de Stirling-PDF (87k★) es "add form entries to PDF" (58 reacciones). Fuera del scope actual de ReVault; nombrarlo como exclusión deliberada, no descuido.

---

## 6. Recomendación

**Para la pregunta del usuario ("si cubrimos el 85%, no tiene sentido hacer más"):**

1. **ReVault ya cubre la mayoría del 85%.** No sobre-construir. El feature-set actual es el correcto para el público objetivo (optimizar/limpiar fotos-videos-PDFs que ya tienes, local).
2. **Si se añade algo, en este orden de ROI (✎ estimaciones corregidas tras verificación; nota: imagen↔PDF NO es simétrico en coste):**
   - **jpg→pdf** — alta demanda, cero deps nuevas (activar feature `embed_image` de lopdf), pero **4-6 días** reales, no 2-3: orientación EXIF (el embed DCTDecode directo la ignora — fotos de móvil saldrían tumbadas), SMask a mano para alpha de PNG (lopdf lo descarta en silencio), fallback CMYK (el código existente lo esquiva, no lo resuelve), mapeo DPI px→pt y opciones de página Fit/A4/Carta+márgenes (estándar en iLovePDF/Smallpdf).
   - **MP4→MP3** — `libmp3lame` verificado en los 4 builds pineados (fuentes primarias + binario local arm64). **~2-3 días**: ffprobe para decidir copy-AAC→.m4a vs. transcode, `-map 0:a:0` explícito, rama para carátula embebida, selector de bitrate. Nota compliance: el binario Linux (johnvansickle) es GPLv3 → source-offer como con gifski.
   - **pdf→jpg** — **~1 semana** con `pdfium-render` + binarios bblanchon (3.2-3.6MB reales, releases inmutables → pin SHA-256 igual que FFmpeg/gifski; evitar los assets `pdfium-v8-*` de 11-27MB). **Requisito de diseño: un solo hilo dueño de Pdfium** (patrón actor) — el feature `thread_safe` de la 0.9.3 tiene un bug de soundness abierto (#262, fix en 0.9.4 sin publicar). Rust puro (hayro) sigue experimental; MuPDF descartado no por AGPL (subprocess = agregación, como gifski) sino porque no tiene ecosistema de binarios pineables.
3. **NO construir:**
   - Pandoc+Tectonic ni LibreOffice para Office↔PDF — alta demanda pero trampa de ROI/fidelidad.
   - El paquete Rust puro del estudio ConvertX (SVG/spreadsheets/EPUB/3D) — apunta a la cola del 15%.
   - Todo el catálogo "1000 formatos" restante — cola confirmada.
4. **Vigilar, no construir aún:** JPEG XL encode (✎ reevaluar cuando Chrome lo active por defecto — hoy: Safari parcial, Firefox release con pref off, Chrome tras flag) y los motores docx→pdf en Rust puro descubiertos en la verificación (dxpdf/Skia, office2pdf/Typst — MIT/Apache, <5MB; hoy fallan en formas/alineación/gráficos según sus propios trackers; reevaluar en 6-12 meses).
5. **Donde probablemente esté el crecimiento real** (según el hallazgo transversal): pulido/UX, batch/automatización, fiabilidad — no más formatos.

**Límite de confianza:** ninguna app publica telemetría; esto triangula proxies. Datos duros más fuertes: Smallpdf 69% de 3 tools, Ahrefs pdf-to-word 220K/mo, escala de TinyPNG, app iOS compress 2M+ usuarios. Puntos ciegos: HEIC↔JPG y PNG↔JPG no tienen respaldo numérico (solo ubicuidad cualitativa); datos de video los más finos.

---

## 7. Verificación adversarial (2026-07-17) — resumen de veredictos

5 subagentes con la misión explícita de refutar cada afirmación load-bearing. Resultado: **el enfoque sobrevive**; correcciones aplicadas inline (✎).

| Afirmación | Veredicto | Corrección clave |
|---|---|---|
| Números de demanda (Smallpdf 34/19/16%, Ahrefs 220K/KD81 + Adobe ~385K, orden grid iLovePDF, formatos TinyPNG) | **CONFIRMADOS verbatim** contra fuente primaria (HTML crudo, no summarizers) | Ahrefs es internamente inconsistente (385K en prosa vs. 402K en su propia tabla) — irrelevante para la conclusión. |
| "mp3 converter ~246K/mo #1 de CloudConvert" | **NO VERIFICABLE** | Semrush gated. Degradado a conjetura de magnitud; la demanda de categoría sigue siendo real. |
| JPEG XL en navegadores | **PARCIALMENTE REFUTADO** | Safari = parcial; Firefox 152 = pref desactivado en release (solo Nightly); Chrome 145 = flag, default-on sin fuente primaria. Watch suavizado. |
| jpg→pdf "2-3 días, cero deps" | Deps **CONFIRMADO** / plazo **REFUTADO** | 4-6 días para calidad competitiva: EXIF orientation (DCTDecode directo la ignora), SMask para alpha PNG, CMYK fallback, DPI px→pt, opciones de página. `compress_pdf_images` esquiva CMYK/SMask, no los resuelve. |
| MP4→MP3 "casi gratis" | Encoder **CONFIRMADO** (4/4 builds) / "gratis" **MATIZADO** | GyanD essentials, evermeet (API JSON propia), johnvansickle (readme adyacente) y arm64 local: todos con libmp3lame. Pero `-c:a copy` ingenuo no es robusto: ffprobe + `-map` + carátula + fallback. Linux binary GPLv3 → source-offer. |
| pdf→jpg con pdfium-render | **CONFIRMADO con 2 correcciones** | (1) Bug de soundness abierto #262 en 0.9.3 (`thread_safe` default miente sobre Send+Sync) → un solo hilo dueño de Pdfium hasta 0.9.4+ verificado. (2) Presupuestar ~10 licencias third-party del binario. Binarios 3.2-3.6MB, releases inmutables, pineables. |
| Office↔PDF "no construir" | **CONFIRMADO Y REFORZADO** | x2t de OnlyOffice = AGPL (muerto por licencia); LibreOffice 293-345MB y `--headless` **crashea en macOS** (bindings Rust lo declaran no soportado); Stirling-PDF/ConvertX con LibreOffice **tienen pdf→docx roto en producción** (issues abiertos) — ni pagando el peso se compra calidad. Word COM = solo bonus oportunista, patrón no soportado por MS. Novedad: ola 2026 de docx→pdf Rust puro (dxpdf, office2pdf) → watch-list 6-12 meses, no shippable hoy. |

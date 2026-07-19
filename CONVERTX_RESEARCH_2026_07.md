# Investigación: ¿Deberíamos incorporar lo que hace ConvertX a ReVault?

**Fecha de investigación:** 2026-07-15
**Estado:** Investigación completa, sin implementación. Documento de referencia, no es roadmap comprometido.
**⚠️ Parcialmente superado (2026-07-17):** el estudio de demanda posterior (`USAGE_DEMAND_STUDY_2026_07.md`) invalidó dos recomendaciones de este doc: el "paquete Rust puro" (§9.1: SVG/spreadsheets/EPUB/3D) apunta a la cola del 15% de uso real — no priorizar; y Pandoc+Tectonic (§9.2) queda descartado también con demanda (re-maqueta, no preserva layout). Siguen vigentes: los descartes de LibreOffice/Calibre/Assimp/ConvertX-embebido, las mediciones de tamaño (§5), el hallazgo `strip = true` pendiente, y el gap de FFmpeg Linux arm64.
**Disparador:** Evaluar si https://github.com/C4illin/ConvertX (self-hosted, 1000+ formatos) tiene sentido como inspiración/fuente de features para ReVault.

---

## 1. Resumen ejecutivo

- **ConvertX y ReVault son productos distintos.** ConvertX es un conversor universal de formatos self-hosted (servidor web, login, Docker). ReVault es una app de escritorio local-only enfocada en optimización + privacidad para un usuario casual.
- **No tiene sentido clonar ConvertX entero.** Arquitectura opuesta (servidor vs. desktop), licencia AGPL-3.0, y el tamaño de sus dependencias (LibreOffice, Calibre) rompe cualquier presupuesto de instalador razonable.
- **Sí tiene sentido tomar piezas puntuales.** Con un presupuesto de **300MB** de peso total de app, se puede sumar **Pandoc + Tectonic** (dos sidecars nuevos) más un paquete de **crates Rust puras** (SVG, spreadsheets, 3D básico, EPUB) y llegar a cubrir **~80-85% de los casos de uso reales** que un usuario casual pediría — aunque solo ~15-20% del catálogo bruto de "1000+ formatos" que ConvertX promociona.
- **Lo que queda fuera de alcance pase lo que pase:** fidelidad visual real de Office (necesita LibreOffice, 300-400MB), ebooks con DRM-ish (mobi/azw3, necesita Calibre, 170-330MB), 3D completo (necesita Assimp), y datos/email/contactos (sin demanda del usuario objetivo).

---

## 2. Qué es ConvertX

- **Repo:** github.com/C4illin/ConvertX — 17.2k estrellas, activo.
- **Qué hace:** subís un archivo a una web self-hosted, elegís formato de salida, lo descargás. Soporta batch, cuentas con login JWT, auto-borrado de archivos tras un tiempo configurable.
- **Stack:** TypeScript + Bun + Elysia + TailwindCSS.
- **Licencia:** AGPL-3.0.
- **Cómo convierte:** no reinventa nada — por cada tipo de archivo llama a una herramienta CLI ya instalada en su contenedor Docker:

| Dominio | Herramienta(s) | Formatos (según su propio README) |
|---|---|---|
| Imagen raster | ImageMagick, GraphicsMagick, Vips, libheif | — |
| Vectorial | Inkscape, resvg, Potrace, VTracer | — |
| Video/audio | FFmpeg | ~472 entrada / ~199 salida |
| Documentos | Pandoc | 43 → 65 |
| Office (fidelidad real) | LibreOffice | 41 → 22 |
| E-books | Calibre | 26 → 19 |
| 3D | Assimp | 77 → 23 |
| LaTeX | XeLaTeX, dvisvgm | — |
| Datos/email/contactos | Markitdown, Dasel, msgconvert | — |

El Dockerfile real de ConvertX (`raw.githubusercontent.com/C4illin/ConvertX/main/Dockerfile`) instala, entre otros: `assimp-utils, calibre, dasel, dcraw, dvisvgm, ffmpeg, ghostscript, graphicsmagick, imagemagick-7.q16, inkscape, latexmk, libheif-examples, libjxl-tools, libreoffice, libvips-tools, libemail-outlook-message-perl, lmodern, mupdf-tools, pandoc, poppler-utils, potrace`, más un **TeXLive completo** y `markitdown[all]`/VTracer compilado desde fuente.

---

## 3. Primer de licencias (por qué importa)

| Licencia | ¿Se puede linkear en `Cargo.toml`? | ¿Se puede usar como sidecar (subprocess)? |
|---|---|---|
| MIT / Apache-2.0 | Sí, sin condiciones | Sí |
| MPL-2.0 | Sí, si no editás los archivos de la crate | Sí |
| GPL-2.0/3.0 | **No** — contamina el binario entero a GPL | Sí, invocación por subprocess = "agregación", no obra combinada |
| AGPL-3.0 | **No** — igual que GPL, más la cláusula de red (solo aplica a servicios de red, ReVault es desktop) | Sí, mismo patrón ya usado con gifski |
| "non-standard" (ej. russimp) | Revisar caso por caso — a veces es solo un flag de metadata, no una restricción real | Generalmente sí |

**Regla aplicada en todo este documento:** cualquier binario GPL/AGPL (Pandoc, Calibre, LibreOffice, Ghostscript) se evalúa únicamente como **sidecar por subprocess**, nunca como dependencia de Rust — mismo patrón ya en producción con FFmpeg y gifski (auto-descarga en el primer uso, no van en el instalador base).

---

## 4. Opciones evaluadas (investigación con 6 subagentes en paralelo)

### 4.1 Rust puro (sin sidecars)

| Formato | Crate(s) | Licencia | Mantenimiento | Veredicto |
|---|---|---|---|---|
| SVG → raster | `resvg` + `usvg` v0.47.0 | Apache-2.0/MIT | Excelente — 20.9M descargas, último push el mismo día de la investigación | API real de librería, ~1 día de esfuerzo |
| Raster → vector | `vtracer` v0.6.5 | MIT/Apache-2.0 | Popular como CLI (6.4k★) pero poco usado como librería (4 reverse deps) | API real (`vtracer::convert`), calidad inferior a Potrace en fotos, ~1 día |
| Spreadsheets (leer) | `calamine` v0.36.0 | MIT | Excelente — 9.9M descargas, última actualización esa semana | Solo lectura, ~1 día |
| Spreadsheets (escribir) | `umya-spreadsheet` v3.0.1 | MIT | Activo pero un solo mantenedor principal | Escribe xlsx válido, sin recalcular fórmulas ni gráficos complejos, ~2 días |
| 3D (OBJ/STL/glTF) | `obj-rs`, `stl_io`, `gltf` | MIT/Apache-2.0 | `gltf` estancado desde 2024-05, **sin capacidad de exportar** | Solo lectura de los 3 formatos; exportar glTF = escribir el exporter a mano (+3-4 días) |
| EPUB (escribir) | `epub-builder` v0.8.3 | MPL-2.0 | Activo | ~1-1.5 días |
| EPUB (leer) | ninguna crate confiable | El único lector maduro (`epub` v2.1.5) es **GPL-3.0 — bloqueante** | — | Hay que escribir un lector propio con `zip`+`quick-xml` (ya usados en el proyecto), ~3-4 días |

**Total estimado:** ~3 semanas, cero impacto en tamaño del instalador, cero riesgo de licencia.

### 4.2 Pandoc como sidecar

- **Licencia:** GPL-2.0-or-later, confirmado. Invocación por subprocess = mismo argumento de "agregación" que ya se usa para gifski.
- **Binarios oficiales (v3.10), tamaños reales verificados vía GitHub API:**

| Plataforma | Tamaño |
|---|---|
| macOS arm64 | 39.5-39.6 MB |
| macOS x64 | 24.6-24.7 MB |
| Windows x64 | 39.3-39.5 MB |
| Linux amd64 | 32.5-33.1 MB |
| Linux arm64 | 34.9-35.4 MB |

- **Cubre nativamente (sin nada más):** docx↔markdown, markdown↔html, odt↔docx, epub↔docx.
- **No cubre:** PDF de salida — Pandoc no tiene motor de PDF propio, necesita `--pdf-engine` externo (LaTeX o wkhtmltopdf).
- **Latencia:** despreciable para uso desktop de un archivo a la vez (regresión conocida solo afecta loops de miles de conversiones/segundo).
- **Obligación de compliance:** mostrar aviso de licencia GPL + link al código fuente en la sección de créditos, igual que ya se hace con gifski.

### 4.3 Tectonic (motor LaTeX, para cerrar el hueco de Pandoc→PDF)

- **Licencia:** MIT.
- **Tamaños reales (v0.16.9, GitHub releases):**

| Plataforma | Tamaño del binario |
|---|---|
| macOS arm64 | 19.6 MB |
| macOS x64 | 19.6 MB |
| Windows x64 (msvc) | 19.1 MB |
| Linux x64 (gnu) | 20.6 MB |

- Motor de TeX autocontenido y embebible, descarga paquetes TeXLive necesarios bajo demanda (no requiere TeXLive completo de varios GB).
- Con esto, Pandoc + Tectonic sí generan PDF real (docx→pdf, md→pdf, latex→pdf) — con matiz: LaTeX re-maqueta el documento, no clona el layout visual exacto del Word original si tenía estilos complejos.

### 4.4 LibreOffice como sidecar — DESCARTADO

- **Licencia:** MPL-2.0 core + componentes LGPLv3+. Sin problema legal vía subprocess.
- **No existe distribución oficial "solo motor headless"**, sin GUI. Terceros (Gotenberg, Stirling-PDF) tampoco lo logran bajar de ~350-400MB — de hecho el tier más liviano de Stirling-PDF **elimina LibreOffice** para llegar a 350MB.
- **Tamaño real:** 300-400MB comprimido, hasta 1.5GB instalado, por plataforma.
- **Cold start:** ~10-12 segundos sin modo daemon; el modo daemon (`--accept=`) necesita un cliente UNO adicional (unoserver/unoconv), con reportes de arranques inestables.
- **Dato relevante:** PDF24 Creator, competidor directo comparable a ReVault, **evita deliberadamente bundlear LibreOffice** por este mismo motivo, usando Ghostscript/PDFBox/QPDF/Tesseract en su lugar.
- **Veredicto: no entra en ningún presupuesto razonable de instalador.**

### 4.5 Calibre como sidecar — DESCARTADO

- **Licencia:** GPL-3.0 confirmado (incluye `ebook-convert`). Vía subprocess, mismo argumento de agregación que gifski — pero GPL exige además poder ofrecer el código fuente del binario bundleado (checklist, no bloqueante).
- **No existe distribución standalone de solo `ebook-convert`** — vive siempre dentro del bundle completo de la app.
- **Tamaños reales verificados (v9.11.0):**

| Plataforma | Tamaño |
|---|---|
| macOS (dmg universal) | 328 MB |
| Windows (MSI) | 213 MB |
| Windows (Portable) | 192 MB |
| Linux x64 | 184 MB |
| Linux arm64 | 172 MB |

- La propia documentación de Calibre advierte que **PDF como formato de origen da resultados pobres** (texto/layout desordenado).
- **Veredicto: no entra en ningún presupuesto razonable de instalador.**

### 4.6 Assimp / `russimp` (3D universal) — DESCARTADO por ahora

- **Licencia:** BSD-3-Clause real, tanto en Assimp como en la copia dentro de `russimp` — el flag "non-standard" en crates.io es solo un artefacto de metadata (`license-file` en vez de `license` SPDX), no una restricción real.
- **`russimp` es solo de lectura** — no expone la API de exportación que sí tiene Assimp en C++. Existe un crate más nuevo (`asset-importer`) sin verificar que podría tener más cobertura.
- Requiere toolchain C++/CMake/bindgen en el pipeline de CI — paso atrás respecto al modelo actual de sidecars binarios puros.
- Tamaño nativo estimado ~20-30MB, pero sin ganancia real (solo lectura, sin demanda de usuario) frente a la alternativa Rust pura (obj-rs/stl_io/gltf, que cubre los 3 formatos más comunes).
- **Veredicto: sin demanda demostrada, no vale el costo de build ni siquiera dentro de presupuesto.**

### 4.7 Embeber ConvertX entero como proceso hijo — DESCARTADO

- Bun sí compila a un ejecutable standalone (`bun build --compile`), pero el runtime de Bun solo ya pesa 57-91MB para un "Hello World" — el propio equipo de Bun documenta que "el binario todavía es demasiado grande."
- **Compilar el código de ConvertX no elimina ninguna dependencia externa.** Su Dockerfile real revela que sigue necesitando LibreOffice + TeXLive completo + Calibre + Assimp + Ghostscript + Inkscape instalados aparte — el mismo peso descartado en 4.4/4.5, sin ahorro alguno.
- **Análisis legal más riesgoso que gifski:** gifski es una invocación de subprocess de una sola pasada (CLI args/pipes). ConvertX sería un **servidor HTTP de larga duración** corriendo dentro de la app, con su propia interfaz web ofrecida al usuario — zona gris real respecto a la cláusula de red del AGPL-3.0 (§13), sin jurisprudencia que lo resuelva con claridad. Requeriría validación legal, no solo una decisión de ingeniería.
- **UX:** su web trae login/JWT propio (sin sentido para un desktop de un solo usuario), sin theming ni i18n compartido con la app nativa Svelte.
- **Veredicto: no ahorra tamaño, no resuelve el riesgo legal, y rompe la identidad de producto.**

---

## 5. Tamaño real medido de ReVault hoy

Medido directamente sobre el build local del repo (no estimado), macOS Apple Silicon, v0.1.0:

| Componente | Tamaño |
|---|---|
| `Revault.app` | 73 MB |
| `.dmg` final | 77-78 MB |
| Frontend compilado (`Contents/Resources`) | 716 KB — insignificante, ventaja de usar el WebView del sistema en vez de empaquetar Chromium |
| Binario Rust (`Contents/MacOS/revault`) | 73 MB — casi todo el peso, por mozjpeg + oxipng/zopfli + ravif linkeados estáticamente |

**Nota de corrección a la memoria del proyecto:** existía una nota de abril 2026 sobre un "objetivo de instalador ~5MB" usada para rechazar `pdfium-render`. Ese número está desactualizado — el build actual ya pesa 73-78MB, 15x más. No usar ese número en decisiones futuras.

**Mejora gratis detectada de paso:** el binario no está compilado con `strip` (~54.800 símbolos de debug presentes). Agregar `strip = true` (+ opcionalmente `lto = true`) en `[profile.release]` de `Cargo.toml` normalmente recorta 20-30% sin tocar ninguna feature. No aplicado, pendiente si se quiere bajar peso.

### Sidecars actuales (FFmpeg + gifski), tamaños reales por plataforma

| Plataforma | FFmpeg (fuente real usada en `core/video.rs`) | gifski |
|---|---|---|
| macOS arm64 | 21.5 + 21.4 MB (self-hosted, GitHub releases del propio repo) = 42.9 MB | 0.6 MB |
| macOS x64 | 26.0 + 25.9 MB (evermeet.cx) = 51.9 MB | 0.7 MB |
| Windows x64 | 104.6 MB (GyanD "essentials_build", un solo zip con extras que no se pueden separar) | 0.6 MB |
| Linux x64 | 39.3 MB (johnvansickle.com static build) | 0.7 MB |

Linux arm64 no tiene fuente de FFmpeg pineada todavía (gap de cobertura existente, no relacionado con esta investigación).

### Total actual (app + FFmpeg + gifski) si el usuario usa Video/GIF

| Plataforma | App | FFmpeg | gifski | **Total** |
|---|---|---|---|---|
| macOS arm64 | 78 MB (medido) | 42.9 MB (medido) | 0.6 MB (medido) | **~121 MB** |
| macOS x64 | ~75 MB (estimado) | 51.9 MB (medido) | 0.7 MB (medido) | **~128 MB** |
| Windows x64 | ~65 MB (estimado) | 104.6 MB (medido) | 0.6 MB (medido) | **~170 MB** |
| Linux x64 | ~70 MB (estimado) | 39.3 MB (medido) | 0.7 MB (medido) | **~110 MB** |

(App en Windows/Linux no se pudo compilar/medir desde este entorno macOS — estimado por analogía de mismo binario Rust y mismas dependencias, mozjpeg sin SIMD en Windows por falta de nasm.)

---

## 6. Escenario con presupuesto de 300MB: sumar Pandoc + Tectonic

Restricción real de diseño: **Windows** es el caso más ajustado (por el FFmpeg de 104.6MB que ahí no se puede achicar) — el presupuesto se diseña contra ese caso para mantener el mismo feature-set en las 4 plataformas.

| Plataforma | App + FFmpeg + gifski (base) | + Pandoc | + Tectonic | **Total final** | Margen bajo 300MB |
|---|---|---|---|---|---|
| Windows x64 | ~170 MB | 39.3 MB | 19.1 MB | **~229 MB** | 71 MB |
| macOS arm64 | ~121 MB | 39.6 MB | 19.6 MB | **~181 MB** | 119 MB |
| macOS x64 | ~128 MB | 24.7 MB | 19.6 MB | **~172 MB** | 128 MB |
| Linux x64 | ~110 MB | 33.1 MB | 20.6 MB | **~164 MB** | 136 MB |

**Conclusión:** con 300MB de techo entran exactamente **2 sidecars nuevos** (Pandoc + Tectonic), con margen de sobra en todas las plataformas. LibreOffice y Calibre siguen sin entrar — cualquiera de los dos, solo, ya rompe el presupuesto entero (habría que subir el techo a 600-700MB+ para meter uno solo).

**Recomendación:** no gastar el margen restante (71-136MB) en Ghostscript/ImageMagick/Assimp solo porque sobra espacio — ninguno tiene demanda demostrada hoy. Guardarlo como colchón para crecimiento futuro (nuevas versiones de FFmpeg, etc.).

---

## 7. Comparación final de cobertura: ConvertX vs. ReVault (con Rust puro + Pandoc + Tectonic)

| Categoría | ConvertX | ReVault (con las sumas propuestas) |
|---|---|---|
| Imagen raster | ImageMagick/GraphicsMagick/Vips (solo convierte) | **Ya lo tiene, y mejor** — pipeline propio que además optimiza calidad/tamaño |
| HEIF/HEIC | libheif (decode + encode) | Decode nativo ya existe. Sin encode (igual que casi todo el ecosistema open source, por patentes) |
| JPEG XL | libjxl-tools (encode + decode) | Solo decode (thumbnails). **Sin encode** |
| Vectorial/SVG | Inkscape + resvg + Potrace/VTracer | resvg+vtracer cubren conversión, no edición tipo Inkscape. Sin AI/EPS/CDR |
| Video/Audio | FFmpeg ~472→199 | FFmpeg ya integrado. Audio standalone: el binario está, falta exponerlo como feature |
| GIF | vía FFmpeg | Ya lo tiene, con mejor calidad (gifski) |
| Documentos simples (docx/odt/html/md/rtf/epub-texto) | Pandoc (43→65) | **Idéntico** — misma herramienta |
| Documentos → PDF | Pandoc+LaTeX o LibreOffice | Cubierto con Tectonic — PDF real, con matiz de re-maquetado |
| Office con fidelidad visual real | LibreOffice (41→22) | **No cubierto** — es la única pieza que exige LibreOffice |
| Spreadsheets | LibreOffice | calamine+umya-spreadsheet cubren el caso común, sin fórmulas recalculadas ni gráficos complejos |
| E-books | Calibre (26→19) | Solo EPUB. **Sin mobi/azw3/fb2/lit** |
| 3D | Assimp (77→23) | Solo lectura de OBJ/STL/glTF, sin export ni los ~74 formatos restantes |
| LaTeX | XeLaTeX + dvisvgm | Tectonic cubre LaTeX→PDF. Sin LaTeX→SVG |
| Datos (JSON/YAML/TOML/XML/CSV) | Dasel | **Cero cobertura** |
| Email (Outlook .msg) | msgconvert | **Cero cobertura** |
| Contactos (VCF/CSV) | vía Markitdown | **Cero cobertura** |
| PDF/Office → Markdown | Markitdown | Parcial — Pandoc lee poco de PDF como entrada |

### Lo que ReVault tiene que ConvertX no tiene en absoluto
- Compresión con control de calidad y predicción de tamaño
- Stripping de privacidad (EXIF/GPS/device, metadata de Office, video Smart/GPS-only/Full)
- Detección de duplicados (SHA/pHash)
- PDF Tools más allá de conversión bruta (merge/split, compress con re-encode de imágenes embebidas, strip metadata)
- Dashboard de ahorro de espacio
- Todo 100% local, sin cuenta, sin servidor

---

## 8. En criollo: qué le falta a ReVault frente a ConvertX (aun sumando todo lo propuesto)

1. **Convertir un Word complicado a PDF pixel-perfect** — el nuestro genera un PDF válido pero re-maquetado (LaTeX), no una copia visual exacta del Word original con sus estilos.
2. **Excel con fórmulas que se recalculen, o con gráficos/tablas dinámicas** — leemos/escribimos el archivo, no recalculamos ni preservamos lo complejo.
3. **Ebooks Kindle (mobi/azw3)** — solo cubrimos EPUB.
4. **Modelos 3D** (ej. un `.fbx` de Blender) — cobertura mínima, solo 3 formatos simples y sin exportar todos.
5. **Correos de Outlook (.msg) o listas de contactos** — cero cobertura.
6. **Datos técnicos entre formatos** (JSON↔YAML↔XML↔CSV) — cero cobertura.
7. **Exportar a JPEG XL** — solo lectura.

**La idea de fondo:** todo lo que falta es terreno de necesidades específicas (diseño 3D, gestión de ebooks, procesamiento masivo de correo, Excel de trabajo complejo) que el usuario objetivo de ReVault (alguien que quiere optimizar fotos/videos del celular, o convertir un documento simple) no pisa. Por eso, aun con presupuesto de sobra, no se recomienda perseguir esos huecos sin demanda real.

---

## 9. Recomendación final

1. **Hacer ahora (bajo riesgo, ~3 semanas):** paquete Rust puro — SVG/vector (`resvg`+`vtracer`), spreadsheets (`calamine`+`umya-spreadsheet`), EPUB (lector propio + `epub-builder`), 3D básico (`obj-rs`/`stl_io`/`gltf`, sin exportar glTF por ahora).
2. **Evaluar después, si aparece demanda real:** Pandoc + Tectonic como sidecars (mismo patrón que FFmpeg/gifski) — suman ~230MB en el peor caso (Windows), dejando la app en, aproximadamente, un tercio del presupuesto de 300MB.
3. **No hacer, ni con presupuesto ampliado:** LibreOffice, Calibre, Assimp/3D completo, embeber ConvertX entero. Ninguno entra en un presupuesto de instalador razonable, y el caso de embeber ConvertX además abre una zona gris legal (AGPL + servidor HTTP) que ni el tamaño resuelve.

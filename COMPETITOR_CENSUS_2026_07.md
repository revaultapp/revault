# Censo de competidores famosos "estilo iLovePDF" — panorama completo

**Fecha:** 2026-07-18
**Estado:** Investigación completa, sin implementación. Documento de referencia, no roadmap.
**Método:** workflow `deep-research` (6 ángulos de búsqueda con subagentes Sonnet → 22 fuentes leídas → ~90 claims extraídas con quotes → verificación adversarial 3 votos/claim). La fase de verificación se cortó a mitad por límite de sesión (26 agentes caídos), así que **los números clave se re-verificaron a mano el 2026-07-18**: estrellas/licencias vía API de GitHub (primario, DATO) y checks dirigidos con curl (PDF24, Convertio). Tráfico web = SimilarWeb (secundario → PROXY siempre).
**Complementa a:** `USAGE_DEMAND_STUDY_2026_07.md` (qué usa la gente) y `CONVERTX_RESEARCH_2026_07.md` (análisis técnico de ConvertX). Este doc responde: *quiénes son los famosos y dónde encajamos*.

> Etiquetas: **DATO** = fuente primaria verificada (web oficial, API GitHub, alerta oficial). **DATO-declarado** = cifra publicada por el propio vendor (verificada la publicación, no la cifra). **PROXY** = señal indirecta real (SimilarWeb, ratings de terceros). **CONJETURA** = inferencia sin fuente primaria verificada hoy.

---

## 1. Resumen ejecutivo

1. **El hueco de ReVault queda CONFIRMADO por el censo**: no existe ningún jugador famoso que cubra imagen+vídeo+PDF junto, con procesado 100% local, cross-platform, open source y gratis. Todos los "locales" famosos son mono-dominio o mono-plataforma (§7).
2. **El modelo dominante es freemium web + Premium ~€5-9/mes** con procesado en servidor (iLovePDF, Smallpdf, TinyPNG, CloudConvert). El SEO orgánico es el 60-68% de su adquisición — la web/landing de ReVault importará más que cualquier feature nueva.
3. **Nadie del censo tiene dedupe, y casi nadie trata la privacidad/metadatos como feature central.** Son los dos diferenciadores de ReVault que no compiten con nadie.
4. **La amenaza más seria es Stirling-PDF** (87.3K★, evoluciona rápido, empaquetado desktop) en el frente PDF, y **VERT** (15.3K★ en ~1 año, WASM local) como concepto "local en el navegador".
5. **Regalo de marketing**: el FBI emitió una alerta oficial (2025-03-17) sobre conversores online falsos que instalan malware, con campañas activas que clonan a players legítimos. "Tus archivos nunca salen de tu máquina" ya no es solo privacidad — es seguridad con respaldo del FBI (§8).

---

## 2. Categoría 1 — Suites PDF web mainstream

| Player | Tráfico/popularidad | Modelo | Procesado | Notas |
|---|---|---|---|---|
| **iLovePDF** | **281.3M visitas/3 meses, rank global #112** (PROXY, SimilarWeb) — el rey absoluto de la categoría | Freemium; Premium €9/mes o €60/año (DATO, pricing oficial) | Servidor (7-11 zonas regionales) | 67.84% tráfico orgánico; India 18%, Brasil 11%, Indonesia 6% (PROXY). Suite completa: iLovePDF + iLoveIMG + iLoveSign + iLoveAPI (DATO) |
| **Smallpdf** | 46.3M visitas/3 meses, rank #875 (PROXY, SimilarWeb) | Freemium + Pro | Servidor | Zurich, fundada 2013, 51-200 empleados (PROXY). Del estudio de demanda previo: 3 tools = 69% de su uso (Compress 34% + Sign 19% + PDF→Word 16%, DATO). ⚠️ La cifra de 46.3M/3mo parece baja vs su percepción histórica — tratar el valor absoluto con cautela, la *forma* (un orden de magnitud bajo iLovePDF) es lo fiable |
| **PDF24** | Sin cifra verificada (los fetches de SimilarWeb fallaron) — CONJETURA: popular en Europa/DACH | **Gratis del todo** (sin premium) | **Web (servidor) + PDF24 Creator OFFLINE en Windows** — verificado hoy: su página repite "Offline" y "free of charge" (DATO) | El único mainstream con un camino offline gratis real; freeware cerrado, Windows-only. Es lo más cercano a ReVault en el frente PDF-desktop-gratis |
| **Sejda** | Sin cifra verificada (bloquea scraping) | Freemium (pases semanales/mes) | Web + Sejda Desktop (CONJETURA: la versión desktop procesa en local — conocimiento previo, no verificado hoy) | Segundo mainstream con camino desktop |
| **Adobe Acrobat online** | Sin cifra propia verificada hoy; del estudio previo: su página PDF→Word ~385K visitas/mes (DATO, verificado entonces) | Freemium → Acrobat Pro | Servidor | La marca ancla de la categoría; su gravedad es la razón de que "pdf to word" sea el keyword #1 |
| **Xodo** | Sin datos verificados — CONJETURA: relevante pero segundo escalón | Freemium | Servidor/apps | Quedó fuera de las fuentes útiles del run |

## 3. Categoría 2 — Conversores universales web

| Player | Señales | Modelo | Notas |
|---|---|---|---|
| **CloudConvert** | 4.7★ (96 reviews SpotSaaS, PROXY); referencia de la categoría para developers | Créditos: free 10/día sin tarjeta; paquetes **one-time desde ~$8** (créditos no caducan) + suscripción; $0.009-0.02/min (DATO pricing + PROXY blog) | Lunaweb GmbH, Alemania, 2010 (PROXY). Free caps: 1GB/archivo, 5 tareas, 5 min (DATO). PDF→Office cuesta hasta 4 créditos — la conversión cara hasta para ellos |
| **Zamzar** | 4.5★ (1,124 reviews, PROXY); pionero (UK, 2006) | Suscripción $9-25/mes (PROXY, fuentes discrepan) | 1,100+ formatos, hoy volcado a su API para developers (PROXY) |
| **Convertio** | "300+ formats" (DATO, verificado hoy con curl) | Freemium | Sin más datos verificados |
| **FreeConvert / Online-Convert** | Sin datos verificados hoy — CONJETURA: tráfico grande, mismo patrón | Freemium + ads | Quedaron fuera de las fuentes útiles del run |

## 4. Categoría 3 — Optimizadores de imagen

| Player | Señales | Procesado | Notas |
|---|---|---|---|
| **TinyPNG** | Clientes declarados: Airbnb, Microsoft, Samsung, Sony, EA (DATO-declarado); del estudio previo ~117K usuarios/mes | **Servidor** — retención de archivos hasta 48h (DATO, web oficial) | ⚠️ Ya comprime y convierte **JXL y AVIF** además de WebP/PNG/JPG/APNG (DATO). Nosotros tenemos JXL solo decode — coherente con nuestra watch-list, pero anotar que el líder ya lo ofrece. Free: 20 imgs × 5MB |
| **Squoosh** | **25,495★ GitHub** (DATO, API hoy), Apache-2.0, **activo** (push 2026-07-17 — no está abandonado, contra el mito) | **100% local en el navegador** — "Images never leave your device" (DATO) | Google Chrome Labs. Gratis, sin batch real, mono-imagen. El referente UX de "local en browser" |
| **iLoveIMG** | Parte del imperio iLove* (DATO) | Servidor (ISO27001/GDPR declarado) | Mismo freemium; compress/resize/convert/upscale/blur-caras |
| **Compressor.io** | Fetch falló — CONJETURA: relevante pero nicho | Servidor | — |

## 5. Categoría 4 — Open source / self-hosted (★ verificadas hoy vía API GitHub, todas DATO)

| Repo | ★ | Licencia | Dominio | ¿Local? |
|---|---|---|---|---|
| **Stirling-PDF** | **87,346** | Open-core (custom) | Solo PDF, 50+ tools (edit, OCR, sign, redact…) | Self-host (Java/Docker) + desktop packaging; "local" pesado, no local-first nativo |
| **Upscayl** | **47,368** | AGPL-3.0 | Solo upscaling AI | 100% local (Electron + Vulkan GPU) |
| **Squoosh** | 25,495 | Apache-2.0 | Solo imagen, mono-archivo | 100% local (browser) |
| **HandBrake** | **23,800** | GPL-2.0 | Solo vídeo (transcode) | 100% local; v1.11.2 jun-2026, activo |
| **ConvertX** | **17,236** | AGPL-3.0 | Universal (1000+ formatos vía LibreOffice/Calibre/FFmpeg/ImageMagick…) | Self-host servidor (Docker); NO local-first — ya analizado en CONVERTX_RESEARCH |
| **VERT** | **15,266** | AGPL-3.0 | Imagen/audio/docs/vídeo, 250+ formatos | **WASM local en browser… EXCEPTO vídeo**: la instancia oficial manda el vídeo a su servidor (daemon `vertd`) (DATO, su propio README). Svelte, joven y creciendo rápido |
| **ImageOptim** | 9,896 | GPL-2.0 | Solo imagen | 100% local, **solo macOS**; 12 motores bundleados (MozJPEG, OxiPNG, Gifsicle…) — validación del enfoque multi-motor de ReVault |

## 6. Categoría 5 — Desktop nativo de pago

| App | Señales | Modelo | Notas |
|---|---|---|---|
| **Permute 4** (Charlie Monroe) | 98% aprobación, 4,303 ratings en Setapp (PROXY) | $14.99 one-time o vía Setapp $14.99/mes (DATO) | El más parecido a ReVault en espíritu multi-dominio (vídeo/audio/imagen/PDF, 100+ formatos)… pero **macOS 26+ Apple Silicon only, cerrado, de pago**, y sin dedupe/privacidad/optimización seria de PDF |
| **PDF Expert** (Readdle) | **30M usuarios declarados** (DATO-declarado); 4.7★ con 211K ratings (DATO-declarado) | Suscripción premium | Apple-only (iPhone/iPad/Mac). Editor PDF premium, no optimizador |
| **Retrobatch / GraphicConverter** | Fetches sin claims — CONJETURA: nicho pro macOS | Pago | Batch de imagen pro, macOS |

---

## 7. Patrones del censo (lo que revela el conjunto)

**Lo que tiene TODO el mundo** (el "precio de entrada", y ReVault ya lo tiene): comprimir imagen y PDF, convertir JPG/PNG/WebP, merge/split PDF, resize. Coincide 1:1 con la cabeza del estudio de demanda.

**Lo que no tiene NADIE del censo** (diferenciadores reales de ReVault):
- **Dedupe**: ningún player famoso lo ofrece. Cero solape competitivo.
- **Privacidad/strip de metadatos como feature central**: TinyPNG lo hace de tapadillo al comprimir; nadie lo vende como herramienta. ReVault tiene página dedicada + modos GPS/dispositivo/fecha/autor.
- **Imagen+vídeo+PDF juntos y locales**: los locales famosos son todos mono-cosa — Squoosh (imagen browser), HandBrake (vídeo), ImageOptim (imagen macOS), Upscayl (upscale), PDF24 Creator (PDF Windows freeware). El multi-dominio local más cercano es Permute: macOS-only, cerrado, de pago, sin dedupe/privacy. **El posicionamiento "único desktop local-only open-source imagen+vídeo+PDF" sobrevive al censo.**

**Estructura del mercado:**
- iLovePDF juega en otra liga de tráfico (~6× Smallpdf en visitas, rank #112 mundial, PROXY).
- 60-68% de la adquisición de los grandes es **búsqueda orgánica** (PROXY consistente en iLovePDF y Smallpdf) → la landing de tryrevault.com con páginas por tarea ("compress pdf offline", "quitar metadatos foto") es el canal, no un nice-to-have.
- El tráfico se concentra en **mercados emergentes** (India + Brasil + Indonesia ≈ 35% en iLovePDF, PROXY) → nuestro i18n en/es/fr apunta bien; pt-BR sería el 4º lógico si algún día ampliamos.
- En OSS, las estrellas premian dos cosas: **cobertura brutal de un dominio** (Stirling 87K) o **"local/privado" como bandera** (Upscayl 47K, VERT 15K en ~1 año). ReVault juega la segunda carta con más dominio que nadie.

## 8. Amenazas y oportunidades

**Amenazas:**
1. **Stirling-PDF** — 87K★, open-core con empresa detrás, ya empaqueta desktop. Si pulen UX no-técnica, comen el frente PDF. Nuestra defensa: ellos son PDF-only, pesados (Java), y ReVault es nativo/ligero multi-dominio.
2. **VERT** — demuestra que "local en el navegador" con WASM es viable y crece rápido; si añaden PDF tools y batch serio, se acercan. Hoy: vídeo va a SU servidor en la instancia oficial (documentado por ellos mismos) — nuestro argumento local es más honesto.
3. **TinyPNG ya vende JXL/AVIF encode** — presión para nuestra watch-list de JXL encode si Chrome lo activa.
4. Los gigantes (iLove*, Smallpdf) ya tienen apps desktop en el tier Premium — si algún día las hacen gratis, el argumento "app de escritorio" se erosiona (el nuestro de verdad es local+open source+gratis, no solo "desktop").

**Oportunidades:**
1. **Seguridad como marketing verificable**: alerta oficial del FBI Denver (2025-03-17, DATO) sobre conversores online falsos que instalan malware; Malwarebytes lista ~10 dominios activos; CloudSEK documenta clones pixel-perfect de players legítimos (candyxpdf[.]com clonando pdfcandy.com) entregando el stealer ArechClient2/SectopRAT (DATO). Narrativa: *"el FBI avisa: los conversores online son un vector de malware — con ReVault nada sale de tu máquina"*. Ningún competidor local-only lo está explotando con fuentes.
2. **SEO por tarea** en tryrevault.com (el 60-68% orgánico de los grandes lo demuestra) — landings "X offline/sin subir archivos".
3. **Dedupe + privacidad**: features sin competencia en el censo; merecen protagonismo en la home/README, no ser bullets del final.
4. **ImageOptim (9.9K★, macOS-only)** valida que hay público para el optimizador local multi-motor; ReVault es "ImageOptim cross-platform + vídeo + PDF".

## 9. Transparencia de verificación

- **Claims re-verificadas a mano tras el corte** (subidas a DATO): las 7 cifras de estrellas/licencias GitHub (API directa 2026-07-18); PDF24 offline/gratis (curl hoy); Convertio 300+ (curl hoy).
- **Refutadas por el panel adversarial** (correctamente): "el tier gratis de iLovePDF es solo-web" (0-3 — sus apps tienen funcionalidad gratuita; Premium las desbloquea completas); "CloudConvert no tiene one-time" (sus Packages SON one-time).
- **Kills dudosos del panel** (votantes degradados por el límite de sesión; mantenidas con matiz): "Stirling corre local como desktop app" (cierto pero es server-Java empaquetado); "Upscayl es 100% local" (su README lo afirma verbatim — lo doy por DATO-declarado).
- **Sin verificar hoy** (CONJETURA hasta nuevo run): Adobe online/Xodo/FreeConvert/Compressor.io/PDF Candy en cifras; Sejda Desktop local-processing; el valor absoluto de tráfico de Smallpdf.

## 10. Implicaciones accionables (sin comprometer roadmap)

1. **No añadir formatos por el censo** — refuerza la conclusión del estudio 85/15: el precio de entrada ya está pagado; nadie gana por catálogo salvo en OSS-PDF (Stirling), donde no competimos por amplitud.
2. La **web de lanzamiento** es el multiplicador (orgánico 60-68%): priorizar landings por tarea + la narrativa FBI/local cuando toque el deploy de tryrevault.com.
3. **Home/README**: subir dedupe y privacidad de rango — son lo que nadie más tiene.
4. **Watch-list sin cambios** (JXL encode cuando Chrome default-on; pdfium 0.9.4; docx→pdf Rust 2027-H1) + añadir: seguir de cerca VERT (¿añaden PDF/batch?) y el desktop de Stirling.

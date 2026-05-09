# ReVault — UX Personality Spec

> **Propósito de este documento:** guía de implementación quirúrgica para dar a ReVault una
> personalidad visual única sin deuda técnica. Cada decisión tiene un porqué. Léelo completo
> antes de tocar cualquier archivo. Un agente que implemente esto sin leer hasta el final
> romperá la coherencia.

---

## Filosofía

ReVault procesa archivos. El usuario repite esto cientos de veces. Toda animación debe
comunicar algo útil — estado, confirmación, valor — nunca decorar por decorar. La regla es:

> **Animación como confirmación, nunca como performance.**

El usuario no debe notar las animaciones conscientemente. Debe notar que la app
*se siente rápida, responsiva y segura*. Si alguien dice "mira qué chula la animación",
hemos fallado. Si dice "esta app se siente muy bien", hemos ganado.

**Timings vigentes en `app.css`** (usarlos siempre, no inventar valores):
```css
--duration-fast:   0.1s
--duration-normal: 0.2s
--duration-slow:   0.3s
--ease-out:        cubic-bezier(0, 0, 0.2, 1)
--ease-in-out:     cubic-bezier(0.4, 0, 0.2, 1)
```

Para animaciones de spring/física usar `svelte/motion` → `Spring` y `Tween`.
Para keyframe loops usar `@keyframes` CSS puro.
**Cero dependencias de animación externas.** motion.dev no tiene soporte oficial Svelte 5.

---

## Las 3 interacciones transversales (PRIORIDAD MÁXIMA)

Estas tres aparecen en TODAS las features (Optimize, Convert, Resize, Privacy, Video, Duplicates).
Son la firma visual de ReVault. Implementarlas primero, hacerlas perfectas, no añadir más
hasta que estén sólidas.

---

### 1. DropZone viva

**Archivo:** `src/lib/components/DropZone.svelte`

El DropZone es la puerta de entrada a cada feature. Si se siente bien aquí, la app se
siente bien desde el primer segundo, en cada pantalla.

**Estado actual del código:**
- `.drop-zone` tiene `border: 2px dashed var(--border)` y `transition: border-color 0.2s, color 0.2s`
- En hover/drag: border cambia a `var(--accent)`, color a `var(--accent)`. Eso es todo.
- `isDragging: $state(false)` ya existe y se activa correctamente con el event de Tauri
- No hay estado de "archivo inválido" — actualmente los archivos malos simplemente se ignoran

**Qué implementar:**

#### A. Borde dashed animado en reposo (idle)

El borde dashed se mueve lentamente, indicando que es una zona activa sin ser ruidoso.
Se detiene cuando hay hover o drag (no dos cosas moviéndose a la vez).

> ⚠️ **NO usar `::before` con `transform: rotate(360deg)`** — `.drop-zone` es un
> contenedor flex con `Upload`, párrafos y `.format-tags` como hijos directos. El
> `::before` con `rotate` arrastraría TODO el contenido interior junto con el borde.
> **La técnica correcta y única es SVG inline** con `stroke-dashoffset`.

**Implementación: SVG `<rect>` inline como primer hijo de `.drop-zone`**

```svelte
<!-- Añadir como primer hijo de <div class="drop-zone"> -->
<svg
  class="border-svg"
  width="100%" height="100%"
  style="position:absolute;inset:0;pointer-events:none;overflow:visible"
>
  <rect
    class="border-rect"
    x="1" y="1"
    width="calc(100% - 2px)" height="calc(100% - 2px)"
    rx="15" ry="15"
    fill="none"
    stroke="var(--border)"
    stroke-width="2"
    stroke-dasharray="8 6"
  />
</svg>
```

```css
/* Quitar el border CSS estático — el SVG lo reemplaza */
.drop-zone {
  border: none;
  position: relative; /* necesario para el SVG absoluto y para ::after del ripple */
}

.border-rect {
  transition: stroke var(--duration-normal) var(--ease-out);
  animation: dash-march 0.8s linear infinite;
}

@keyframes dash-march {
  to { stroke-dashoffset: -14; } /* 8 + 6 = un ciclo completo */
}

/* Parar la marcha en hover/drag */
.empty:hover .border-rect,
.empty.dragging .border-rect {
  animation-play-state: paused;
  stroke: var(--accent);
}

/* Estado inválido */
.empty.invalid .border-rect {
  animation-play-state: paused;
  stroke: var(--danger);
}
```

#### B. Drag-over magnético

Cuando el usuario arrastra archivos sobre el DropZone (estado `isDragging = true`):

```css
.empty.dragging .drop-zone {
  transform: scale(1.02);
  border-color: var(--accent);
  box-shadow: 0 0 0 4px var(--accent-glow), 0 8px 32px rgba(16, 216, 122, 0.12);
  color: var(--accent);
}

/* Transición hacia dragging más rápida que la salida */
.drop-zone {
  transition:
    transform 150ms var(--ease-out),
    box-shadow 200ms var(--ease-out),
    border-color var(--duration-normal) var(--ease-out),
    color var(--duration-normal) var(--ease-out);
}
```

El icono Upload también reacciona:
```css
.empty.dragging .drop-zone :global(svg) {
  transform: translateY(-3px) scale(1.1);
  transition: transform 200ms var(--ease-out);
  color: var(--accent);
}
```

#### C. Confirmación de drop (ripple)

Cuando el usuario suelta los archivos sobre el DropZone, dar feedback visual inmediato.
Requiere añadir `isDropped` e `isInvalid` como `$state` **mutuamente excluyentes** — nunca
pueden estar activos a la vez porque animan la misma propiedad `transform` del `.drop-zone`.

```svelte
<script>
  let isDropped = $state(false);
  let isInvalid = $state(false);
  let dropTimer: ReturnType<typeof setTimeout>;
  let invalidTimer: ReturnType<typeof setTimeout>;

  // En el handler del drop event (bloque 'drop' del onDragDropEvent):
  // const paths = event.payload.paths.filter((p) => acceptedExtensions.test(p));
  // if (paths.length > 0) {
  //   isInvalid = false;                          // exclusión mutua
  //   clearTimeout(invalidTimer);
  //   onfiles(paths);
  //   isDropped = true;
  //   clearTimeout(dropTimer);
  //   dropTimer = setTimeout(() => { isDropped = false; }, 600);
  // } else {
  //   isDropped = false;                          // exclusión mutua
  //   clearTimeout(dropTimer);
  //   isInvalid = true;
  //   clearTimeout(invalidTimer);
  //   invalidTimer = setTimeout(() => { isInvalid = false; }, 400);
  // }
</script>
```

```css
.empty.dropped .drop-zone {
  animation: drop-confirm 600ms var(--ease-out) forwards;
}

@keyframes drop-confirm {
  0%   { transform: scale(0.98); }
  30%  { transform: scale(1.03); }
  60%  { transform: scale(1.01); }
  100% { transform: scale(1); }
}

/* Ring expansivo (::after) */
.empty.dropped .drop-zone::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 16px;
  border: 2px solid var(--accent);
  animation: ring-expand 600ms var(--ease-out) forwards;
  pointer-events: none;
}

@keyframes ring-expand {
  0%   { transform: scale(1); opacity: 0.8; }
  100% { transform: scale(1.06); opacity: 0; }
}
```

#### D. Archivo inválido (shake + feedback)

**Importante:** actualmente los archivos inválidos se filtran silenciosamente. El usuario
no sabe qué pasó. La lógica de `isInvalid` ya está descrita en el bloque C — con
`clearTimeout(invalidTimer)` para evitar la race condition si el usuario arrastra inválidos
dos veces seguidas en menos de 400ms.

```css
.empty.invalid .drop-zone {
  animation: shake 400ms var(--ease-out);
  border-color: var(--danger);
  box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.12);
}

@keyframes shake {
  0%, 100% { transform: translateX(0); }
  20%       { transform: translateX(-5px); }
  40%       { transform: translateX(5px); }
  60%       { transform: translateX(-3px); }
  80%       { transform: translateX(3px); }
}
```

Añadir también un texto inline que aparece 200ms después del shake:
```svelte
{#if isInvalid}
  <p class="invalid-msg" transition:fade={{ duration: 200 }}>
    Format not supported
  </p>
{/if}
```

**Nunca usar color como único indicador** — la combinación shake + color + texto es
la señal accesible completa.

---

### 2. Transición processing → results

> ⚠️ **Hallazgo de auditoría (2026-05-09):** el switch de 3 estados NO vive en cada
> `*Page.svelte`. Vive en **`ToolShell.svelte`**, que es el componente compartido por las
> 5 features (Optimize, Convert, Resize, Privacy, Video). Implementar las transiciones
> **una sola vez en `ToolShell.svelte`** y todas las features las heredan gratis.
> No tocar los `*Page.svelte` para este punto.

**Archivo:** `src/lib/components/ToolShell.svelte`

El switch actual usa dos condiciones booleanas (no un enum `phase`):
```
{#if files.length === 0}   → idle (DropZone)
{:else if isProcessing}    → processing (ProgressRing)
{:else}                    → results (.tool-view)
```
Las transiciones Svelte funcionan igual con `{#if}/{:else if}` — no hace falta
refactorizar a un enum `phase`.

Actualmente el cambio de estado es un swap brusco. Este es EL momento de payoff de
cada operación. Si se siente bien aquí, la app se siente premium en todas partes.

**Patrón a implementar en `ToolShell.svelte`:**

Añadir `import { fly, fade, scale } from 'svelte/transition'` y `import { cubicOut } from 'svelte/easing'`.
Envolver cada bloque del `{#if}` con un `<div>` que lleve las directivas de transición:

```svelte
{#if files.length === 0}
  <!-- idle — DropZone desaparece rápido -->
  <div out:fade={{ duration: 150 }}>
    <DropZone {onfiles} ... />
  </div>

{:else if isProcessing}
  <!-- processing — ProgressRing entra con scale, sale encogiendo -->
  <div
    in:scale={{ duration: 200, start: 0.95, easing: cubicOut }}
    out:scale={{ duration: 250, start: 1, opacity: 0 }}
  >
    <ProgressRing targetPct={$targetPct} label={$statusLabel} sublabel={$savingsLabel} />
  </div>

{:else}
  <!-- results — entra desde abajo con fly suave -->
  <div
    in:fly={{ y: 12, duration: 280, easing: cubicOut }}
    class="tool-view"
  >
    <!-- contenido existente de resultados sin cambios -->
  </div>
{/if}
```

**Timing crítico:** el `out` del ProgressRing y el `in` del results se solapan
automáticamente porque Svelte corre ambas transiciones en paralelo cuando el bloque
`{#if}` cambia. No hace falta configuración extra.

**Importante:** los nombres exactos de las variables (`$targetPct`, `$statusLabel`, etc.)
dependen de cómo `ToolShell` recibe sus props. Leer el componente completo antes de
implementar para usar los nombres correctos, no los del ejemplo.

**El ProgressRing al completar (100%):**

Añadir en `ProgressRing.svelte` un estado de completado que se dispara cuando
`displayPct >= 99.9`. El número "100%" hace crossfade a un checkmark.

```svelte
<script>
  let isComplete = $derived(displayPct >= 99.9);
</script>

<span class="pct" class:complete={isComplete}>
  {#if isComplete}
    <!-- CheckCircle icon de lucide-svelte, tamaño 42 -->
    <span in:scale={{ duration: 300, start: 0.6, easing: cubicOut }} class="check-icon">
      <CheckCircle size={42} strokeWidth={1.5} color="var(--accent)" />
    </span>
  {:else}
    {Math.round(displayPct)}<small>%</small>
  {/if}
</span>
```

El SVG del ring al completar:
```css
/* Cuando displayPct >= 99.9 — añadir clase 'complete' al circle-wrap */
.circle-wrap.complete svg {
  animation: completion-burst 500ms var(--ease-out) forwards;
}

@keyframes completion-burst {
  0%   { transform: scale(1);    filter: drop-shadow(0 0 8px rgba(16, 216, 122, 0.2)); }
  40%  { transform: scale(1.06); filter: drop-shadow(0 0 20px rgba(16, 216, 122, 0.6)); }
  70%  { transform: scale(1.03); filter: drop-shadow(0 0 14px rgba(16, 216, 122, 0.4)); }
  100% { transform: scale(1);    filter: drop-shadow(0 0 10px rgba(16, 216, 122, 0.25)); }
}
```

Ring expansivo de completado (elemento `::after` en `.circle-wrap`):
```css
.circle-wrap.complete::after {
  content: '';
  position: absolute;
  width: 180px;
  height: 180px;
  border-radius: 50%;
  border: 2px solid var(--accent);
  animation: ring-fade-out 600ms var(--ease-out) forwards;
  pointer-events: none;
}

@keyframes ring-fade-out {
  0%   { transform: scale(1);    opacity: 0.7; }
  100% { transform: scale(1.35); opacity: 0; }
}
```

---

### 3. Savings counter vivo

**Archivo:** `src/lib/components/Sidebar.svelte`

El `.saved-badge` actualmente muestra `formatBytes($savings.totalSavedBytes)` estático.
Cuando cualquier operación completa y llama `savings.add(bytes)`, el número en el sidebar
debe animar. Este es el corazón emocional de ReVault: el usuario ve que su disco se libera
en tiempo real.

**Implementación con `svelte/motion` → `Tween`:**

```svelte
<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { savings } from '$lib/stores/savings';
  import { formatBytes } from '$lib/utils';

  // Tween que sigue el valor real con suavizado
  const displayedBytes = new Tween(0, {
    duration: 800,
    easing: (t) => t < 0.5 ? 2*t*t : -1+(4-2*t)*t  // ease-in-out cuadrático
  });

  // Reaccionar a cambios en el store — $effect en Svelte 5
  $effect(() => {
    displayedBytes.set($savings.totalSavedBytes);
  });
</script>

<!-- En el template, reemplazar el span del savings: -->
<div class="saved-badge" class:just-updated={savingsJustUpdated}>
  <Database size={16} strokeWidth={1.8} />
  <span class="savings-value">{formatBytes($displayedBytes.current)}</span>
</div>
```

**El glow al actualizarse:**

Añadir `savingsJustUpdated: $state(false)` que se activa cuando el store sube y se
resetea tras 1200ms:

```css
.saved-badge {
  /* estilos existentes... */
  transition: box-shadow 300ms var(--ease-out);
}

.saved-badge.just-updated {
  box-shadow: 0 0 0 3px rgba(16, 216, 122, 0.25), 0 0 16px rgba(16, 216, 122, 0.15);
  animation: savings-pulse 1200ms var(--ease-out) forwards;
}

@keyframes savings-pulse {
  0%   { box-shadow: 0 0 0 3px rgba(16, 216, 122, 0.35), 0 0 20px rgba(16, 216, 122, 0.2); }
  100% { box-shadow: 0 0 0 0px rgba(16, 216, 122, 0), 0 0 0px rgba(16, 216, 122, 0); }
}
```

**Importante sobre `formatBytes`:** el número animado pasará por valores intermedios
(e.g., "1.23 MB", "1.45 MB"). `formatBytes` debe recibir el valor numérico entero y formatear
con suficiente precisión para que el conteo sea visible. Verificar que la función actual en
`utils.ts` acepta floats — si redondea a 0 decimales en MB, el tween no será visible hasta
que cambie la unidad. Ajustar a 1-2 decimales en MB/GB para que el conteo sea perceptible.

---

## Micro-interacciones de soporte

Estas no son transversales — aplican a componentes específicos. Implementar DESPUÉS de
las 3 core. Son la segunda capa de pulido.

### File list — stagger de entrada

**Aplica en:** CompressPage, ConvertPage, ResizePage, PrivacyPage, VideoPage
(cualquier `{#each}` que renderiza la lista de archivos añadidos)

```svelte
<script>
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
</script>

{#each files as file, i (file.path)}
  <div
    in:fly={{ y: 8, opacity: 0, duration: 220, delay: Math.min(i, 9) * 40, easing: cubicOut }}
  >
    <!-- row del archivo -->
  </div>
{/each}
```

**Cap en 10 filas** (`Math.min(i, 9) * 40`): los archivos del índice 10 en adelante aparecen
sin delay. En batches grandes (50+ archivos), el stagger se volvería irritante si cubre
todos los items.

**NO usar stagger en la lista de resultados** si los items llegan de forma asíncrona
(uno por uno conforme terminan). En ese caso, cada fila entra con `in:fly` pero sin delay
calculado por índice — el timing natural de la completación es suficiente.

### Sidebar — active indicator deslizante

**Archivo:** `src/lib/components/Sidebar.svelte`

Actualmente el `.accent-bar` se renderiza con `{#if $activePage === item.id}` —
aparece y desaparece bruscamente. El indicador debería deslizarse entre items.

**Técnica con `svelte/motion` → `Spring`:**

```svelte
<script lang="ts">
  import { Spring } from 'svelte/motion';
  import { activePage } from '$lib/stores/nav';

  // Referencias DOM para medir posición Y de cada item
  let navRefs: Record<string, HTMLElement> = {};
  let navEl: HTMLElement;

  const indicatorY = new Spring(0, { stiffness: 0.3, damping: 0.8 });
  let indicatorVisible = $state(false);

  $effect(() => {
    const activeRef = navRefs[$activePage];
    const navRect = navEl?.getBoundingClientRect();
    if (activeRef && navRect) {
      const itemRect = activeRef.getBoundingClientRect();
      indicatorY.set(itemRect.top - navRect.top + (itemRect.height / 2) - 14);
      indicatorVisible = true;
    }
  });
</script>

<nav class="nav" bind:this={navEl}>
  <!-- Indicador deslizante — posición absoluta -->
  {#if indicatorVisible}
    <span
      class="sliding-indicator"
      style="transform: translateY({$indicatorY.current}px)"
    ></span>
  {/if}

  {#each navItems as item (item.id)}
    <button
      bind:this={navRefs[item.id]}
      class="nav-item"
      class:active={$activePage === item.id}
      onclick={() => activePage.set(item.id)}
    >
      <!-- Quitar el accent-bar estático dentro del botón -->
      <item.icon size={18} strokeWidth={1.8} />
      <span>{item.label}</span>
    </button>
  {/each}
</nav>
```

```css
.sliding-indicator {
  position: absolute;
  left: 0;
  width: 3px;
  height: 28px;
  border-radius: 2px;
  background: var(--accent);
  pointer-events: none;
  will-change: transform;
  /* translateY viene del Spring */
}

.nav {
  position: relative; /* necesario para absolute del indicator */
}
```

**Si la implementación con Spring resulta compleja** (problema de medición en el primer render,
race conditions con onMount), la alternativa aceptable es mantener el `{#if}` actual pero
añadir `transition:scale={{ duration: 200, start: 0.7 }}` al `.accent-bar`. Es menos elegante
pero sin riesgo de bugs de layout.

### Button — estado loading y success

**Archivo:** `src/lib/components/Button.svelte`

El Button actual ya tiene `translateY(-1px)` hover y `scale(0.98)` active. Lo que falta
es feedback de operaciones asíncronas.

Añadir props `loading` y `success`:

```svelte
<script>
  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "ghost";
    size?: "sm" | "md";
    danger?: boolean;
    loading?: boolean;    // NUEVO
    success?: boolean;    // NUEVO
    alignSelf?: string;
    children: Snippet;
  }

  let { loading = false, success = false, ...rest } = $props();
</script>

<button
  class={buttonClass}
  class:loading={loading}
  class:success={success}
  disabled={loading || rest.disabled}
  {...rest}
>
  {#if loading}
    <span class="spinner" aria-hidden="true"></span>
  {:else if success}
    <span in:scale={{ duration: 280, start: 0.5, easing: cubicOut }}>
      <!-- CheckIcon de lucide-svelte, size 14 -->
    </span>
  {/if}
  {@render children()}
</button>
```

```css
.btn-primary.loading {
  opacity: 0.7;
  cursor: wait;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 600ms linear infinite;
  flex-shrink: 0;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.btn-primary.success {
  background: var(--accent-hover);
}
```

**Uso típico:** el botón "Compress" en CompressPage pasa a `loading=true` mientras procesa
y a `success=true` al completar (durante 1.5s antes de volver a idle).

---

## Privacy chip post-batch

Este es el "signature moment" #3 — el más diferenciador. Aparece SOLO cuando una operación
ha extraído/eliminado metadatos GPS o EXIF.

**Archivo nuevo:** `src/lib/components/PrivacyToast.svelte`

```svelte
<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { Shield } from 'lucide-svelte';

  interface Props {
    message: string;       // "GPS removed from 23 files"
    visible: boolean;
  }

  let { message, visible } = $props();
</script>

{#if visible}
  <div
    class="privacy-chip"
    in:fly={{ y: 10, duration: 350, easing: cubicOut }}
    out:fade={{ duration: 250 }}
    role="status"
    aria-live="polite"
  >
    <Shield size={14} strokeWidth={2} />
    <span>{message}</span>
  </div>
{/if}

<style>
  .privacy-chip {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 20px;
    background: var(--bg-card);
    border: 1px solid rgba(16, 216, 122, 0.25);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(16, 216, 122, 0.1);
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
    pointer-events: none;
    z-index: 100;
  }
</style>
```

**Lógica:** cada página que hace privacy stripping (PrivacyPage, CompressPage con strip toggle,
VideoPage con PrivacyMode != Off) cuenta los archivos procesados con GPS/EXIF y muestra el chip
durante 3000ms al completar. El mensaje varía:
- 1 archivo: `"Metadata removed from 1 file"`
- N archivos: `"GPS removed from {n} files"`
- Video con PrivacyMode.Smart: `"Metadata stripped (GPS preserved)"`

---

## Respeta `prefers-reduced-motion`

**Crítico.** La memoria del proyecto indica que `prefers-reduced-motion` solo se respeta en
VideoPage actualmente. Debe aplicarse a TODAS las animaciones de este spec.

**Svelte nativo:** `svelte/motion` (`Spring`, `Tween`) ya respetan `prefersReducedMotion`
desde Svelte 5.7.0 automáticamente si se usa el helper:

```svelte
<script>
  import { prefersReducedMotion } from 'svelte/motion';
</script>
```

**CSS `@keyframes`:** añadir en `app.css` global:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

**Transiciones Svelte `in:`/`out:`:** pasar `duration: 0` cuando `prefersReducedMotion.current`:

```svelte
<script>
  import { prefersReducedMotion } from 'svelte/motion';
  const dur = $derived(prefersReducedMotion.current ? 0 : 280);
</script>

<div in:fly={{ y: 8, duration: dur }}>...</div>
```

---

## Lo que NO implementar

Estas ideas pueden parecer atractivas pero perjudican la UX en uso repetitivo:

- **Confetti o celebraciones visuales explosivas** — la app procesa archivos en bucle. La
  celebración se convierte en ruido al archivo #20.
- **Page transitions largas (>300ms) entre features del sidebar** — el usuario navega muchas
  veces. La ceremonia enlentece el flujo mental.
- **Skeleton shimmer en listas de archivos** — el procesamiento es local e instantáneo.
  El shimmer implica latencia de red que no existe. Es engañoso.
- **Animaciones de hover en iconos del sidebar que rotan o hacen flip** — demasiado juguetón
  para una tool app seria.
- **Spring physics con bounce exagerado** (`stiffness > 0.5, damping < 0.5`) — hace que la
  app parezca un juego, no una herramienta.
- **Número de savings con 0 decimales en MB/GB** — si el Tween va de 1.20 a 1.25 GB y
  `formatBytes` muestra "1 GB" todo el rato, la animación es invisible. Usar 1-2 decimales.
- **Stagger sin cap** en listas grandes — `i * 40ms` para 50 archivos = 2 segundos de espera.
  El cap en 10 filas es obligatorio.
- **DropZone dashed border rotando durante el drag-over** — dos cosas moviéndose a la vez
  compiten por atención. Pausar la rotación cuando hay drag-over.
- **Animaciones que modifiquen `width` o `height`** — siempre usar `transform: scale()` para
  evitar reflows. Las únicas propiedades seguras para animar son `transform`, `opacity`,
  `filter`, y `clip-path`.

---

## Stack de implementación — resumen ejecutivo

| Interacción | Técnica | Dependencia |
|---|---|---|
| DropZone border dashed | CSS `@keyframes` | Ninguna |
| DropZone drag-over scale+glow | CSS `transition` + clase JS | Ninguna |
| DropZone drop ripple | CSS `@keyframes` + clase JS | Ninguna |
| DropZone shake inválido | CSS `@keyframes` + clase JS | Ninguna |
| File list stagger | `in:fly` con delay por índice | `svelte/transition` (built-in) |
| Processing → results swap | `in:fly` / `out:scale` | `svelte/transition` (built-in) |
| ProgressRing completion burst | CSS `@keyframes` + clase `$derived` | Ninguna |
| ProgressRing % → checkmark | `in:scale` transition | `svelte/transition` (built-in) |
| Savings counter animado | `Tween` de `svelte/motion` | `svelte/motion` (built-in) |
| Savings badge glow | CSS `@keyframes` + clase JS | Ninguna |
| Sidebar active indicator | `Spring` de `svelte/motion` | `svelte/motion` (built-in) |
| Button loading spinner | CSS `@keyframes` | Ninguna |
| Button success checkmark | `in:scale` transition | `svelte/transition` (built-in) |
| Privacy chip | `in:fly` / `out:fade` | `svelte/transition` (built-in) |

**Total dependencias nuevas: 0.** Todo es Svelte built-in o CSS puro.

---

## Orden de implementación recomendado

Implementar en este orden. Cada paso está completo y testeable antes del siguiente.

```
Sprint 1 — Las 3 transversales:
  1. DropZone: SVG border animado + hover magnético + drop ripple + shake inválido
     Archivos: DropZone.svelte
     Técnica borde: SVG inline con stroke-dashoffset (NO ::before rotate)
     Race conditions: clearTimeout antes de cada setTimeout; isDropped/isInvalid mutuamente excluyentes
     Test: drag-over, drop válido, drop inválido, drop inválido x2 rápido (<400ms)

  2. Processing → results transition
     Archivo: ToolShell.svelte (UNO solo — beneficia las 5 features automáticamente)
     NO tocar los *Page.svelte individuales para este punto
     Test: comprimir 1 imagen, verificar transición fluida en Optimize + Privacy + Video

  3. Savings counter animado
     Archivos: Sidebar.svelte (Tween + glow)
     Test: comprimir imagen, verificar que el counter sube animado con glow

Sprint 2 — Micro-interacciones de soporte:
  4. ProgressRing completion burst + % → checkmark
     Archivos: ProgressRing.svelte
     
  5. File list stagger
     Archivos: CompressPage.svelte (piloto), luego otros *Page
     
  6. Privacy chip post-batch
     Archivos: PrivacyToast.svelte (nuevo), integrar en PrivacyPage + CompressPage
     
Sprint 3 — Pulido:
  7. Sidebar active indicator deslizante (Spring)
     Archivos: Sidebar.svelte
     Añadir guard: `if (!navEl) return` en el $effect antes de llamar getBoundingClientRect()
     Usar `tick()` de svelte antes de medir el DOM para garantizar que bind:this resolvió
     Alternativa segura si persisten race conditions: mantener accent-bar estático pero con
     `transition:scale={{ duration: 200, start: 0.7 }}` — menos elegante, cero riesgo de bugs
     
  8. Button loading + success states
     Archivos: Button.svelte + actualizar callers en todas las *Page
     
  9. DropZone dashed border animado en idle
     Archivos: DropZone.svelte (técnica SVG inline preferida)
     Nota: dejar para último porque es la más compleja de los 3 estados del DropZone
     
  10. prefers-reduced-motion global en app.css
      Archivos: app.css
      Nota: hacer SIEMPRE antes de mergear a main, no al final del sprint
```

---

## Variables CSS clave (no inventar nuevas)

```css
/* Accent */
--accent:        #10D87A
--accent-glow:   rgba(16, 216, 122, 0.15)
--accent-subtle: rgba(16, 216, 122, 0.08)
--accent-hover:  color-mix(in oklch, var(--accent) 85%, black)

/* Danger */
--danger:    #ef4444
--danger-bg: rgba(239, 68, 68, 0.1)    /* light */  / rgba(239, 68, 68, 0.15)  /* dark */

/* Fondos */
--bg-card:   #ffffff  /* light */  / #0c0f0e  /* dark */
--navy-bg:   #e4ecf2  /* light */  / #141918  /* dark */

/* Timings */
--duration-fast:   0.1s
--duration-normal: 0.2s
--duration-slow:   0.3s
--ease-out:        cubic-bezier(0, 0, 0.2, 1)
--ease-in-out:     cubic-bezier(0.4, 0, 0.2, 1)
```

Si se necesita un glow más intenso que `--accent-glow` para el completion burst,
usar `rgba(16, 216, 122, 0.4)` inline — no crear una variable nueva solo para eso.

---

## Verificación pre-merge

Antes de mergear cualquier sprint a `main`:

- [ ] Todas las animaciones respetan `prefers-reduced-motion`
- [ ] Ninguna animación usa `width`/`height` — solo `transform`/`opacity`/`filter`/`clip-path`
- [ ] File list stagger tiene cap en 10 filas (`Math.min(i, 9)`)
- [ ] DropZone dashed border pausa durante drag-over (no dos cosas moviéndose a la vez)
- [ ] `pnpm check` sin errores TypeScript
- [ ] `cargo clippy` sin warnings (si hay cambios en Rust, improbable aquí)
- [ ] Test manual en dark mode Y light mode — verificar que glow accent es visible en ambos
- [ ] Test manual con 1 archivo y con 20 archivos — verificar que el stagger no enlentece el UX

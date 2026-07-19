# Dashboard Complete Data UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar el Dashboard poblado aprobado con tendencias por categoría, último escaneo escalable y layout responsive usando únicamente datos reales.

**Architecture:** La lógica determinista de agrupación y navegación vive en `src/lib/charts.ts`; los componentes Svelte consumen props y gestionan sólo selección/previsualización. `DashboardPage.svelte` sigue orquestando stores e i18n, y las container queries resuelven el layout según el espacio real disponible.

**Tech Stack:** Svelte 5 runes, TypeScript 6, Vitest/jsdom, CSS scoped con tokens de `src/app.css`, Lucide Svelte.

---

## Mapa de archivos

- `src/lib/charts.ts`: tipos y helper puro para ordenar/agrupar segmentos visuales del donut; reutiliza la navegación de índices existente.
- `src/lib/charts.test.ts`: contratos de agrupación sin pérdida y casos límite.
- `src/lib/utils.ts`: formateador de bytes localizado sin cambiar el contrato existente de `formatBytes`.
- `src/lib/utils.test.ts`: contratos EN/ES/DE del nuevo formateador.
- `src/lib/components/CategoryLines.svelte`: selección estable por mes, resumen persistente, líneas distinguibles y responsive propio.
- `src/lib/components/CategoryLines.test.ts`: interacción por teclado, hover, reordenación, tabla y semántica.
- `src/lib/components/StorageDonut.svelte`: donut limitado a cinco segmentos visuales, leyenda canónica y tabla completa.
- `src/lib/components/StorageDonut.test.ts`: agrupación integrada, teclado/puntero, tabla completa y semántica.
- `src/lib/components/DashboardPage.svelte`: headings asociados, props completas y container queries del panel.
- `src/lib/i18n/locales/{en,es,fr,de,pt}.ts`: etiquetas nuevas con paridad entre idiomas.
- `src/lib/stores/locale.test.ts`: la prueba existente validará paridad de claves.
- `docs/superpowers/specs/2026-07-19-dashboard-complete-data-ui-design.md`: fuente funcional aprobada.

### Task 1: Agrupación visual del último escaneo

**Files:**
- Modify: `src/lib/charts.ts`
- Modify: `src/lib/charts.test.ts`

- [ ] **Step 1: escribir pruebas fallidas para orden y “Otros”**

Añadir imports y casos con este contrato:

```ts
import { groupDonutDisplaySegments } from "./charts";

const raw = [
  { label: "PNG", bytes: 20, count: 2 },
  { label: "JPG", bytes: 50, count: 5 },
  { label: "HEIC", bytes: 10, count: 1 },
  { label: "WEBP", bytes: 15, count: 3 },
  { label: "AVIF", bytes: 5, count: 1 },
  { label: "BMP", bytes: 2, count: 4 },
];

expect(groupDonutDisplaySegments(raw, "Other", 5)).toEqual([
  { key: "jpg", label: "JPG", bytes: 50, count: 5, sourceLabels: ["JPG"] },
  { key: "png", label: "PNG", bytes: 20, count: 2, sourceLabels: ["PNG"] },
  { key: "webp", label: "WEBP", bytes: 15, count: 3, sourceLabels: ["WEBP"] },
  { key: "heic", label: "HEIC", bytes: 10, count: 1, sourceLabels: ["HEIC"] },
  { key: "__other__", label: "Other", bytes: 7, count: 5, sourceLabels: ["AVIF", "BMP"] },
]);
```

Incluir casos para vacío, uno y cinco segmentos, además de verificar que inputs inválidos/negativos no producen arcos negativos.

- [ ] **Step 2: ejecutar RED**

Run: `pnpm exec vitest run src/lib/charts.test.ts`
Expected: FAIL porque `groupDonutDisplaySegments` todavía no existe.

- [ ] **Step 3: implementar el helper mínimo**

Añadir tipos exportados y función pura:

```ts
export interface RawChartSegment {
  label: string;
  bytes: number;
  count: number;
}

export interface DisplayChartSegment extends RawChartSegment {
  key: string;
  sourceLabels: string[];
}

export function groupDonutDisplaySegments(
  segments: RawChartSegment[],
  otherLabel: string,
  maxVisible = 5,
): DisplayChartSegment[] {
  const merged = new Map<string, DisplayChartSegment>();
  for (const segment of segments) {
    if (!Number.isFinite(segment.bytes) || segment.bytes < 0) continue;
    const label = segment.label.trim();
    if (!label) continue;
    const key = label.toLocaleLowerCase();
    const current = merged.get(key);
    merged.set(key, current
      ? { ...current, bytes: current.bytes + segment.bytes, count: current.count + segment.count, sourceLabels: [...current.sourceLabels, segment.label] }
      : { ...segment, label, key, sourceLabels: [segment.label] });
  }
  const normalized = [...merged.values()]
    .sort((a, b) => b.bytes - a.bytes || a.label.localeCompare(b.label));
  if (normalized.length <= maxVisible) return normalized;
  const head = normalized.slice(0, maxVisible - 1);
  const tail = normalized.slice(maxVisible - 1);
  return [...head, {
    key: "__other__",
    label: otherLabel,
    bytes: tail.reduce((sum, segment) => sum + segment.bytes, 0),
    count: tail.reduce((sum, segment) => sum + segment.count, 0),
    sourceLabels: tail.flatMap((segment) => segment.sourceLabels),
  }];
}
```

Etiquetas vacías se omiten y etiquetas que sólo difieren en mayúsculas/minúsculas se fusionan bajo una clave estable.

- [ ] **Step 4: ejecutar GREEN**

Run: `pnpm exec vitest run src/lib/charts.test.ts`
Expected: PASS.

### Task 2: CategoryLines comprensible y navegable

**Files:**
- Create: `src/lib/components/CategoryLines.test.ts`
- Modify: `src/lib/components/CategoryLines.svelte`

- [ ] **Step 1: escribir pruebas fallidas del contrato de interacción**

Montar el componente como `MonthlyBars.test.ts` con puntos que incluyan una `key` estable. Verificar:

```ts
expect(target.querySelectorAll("[role='radio']")).toHaveLength(12);
expect(target.querySelectorAll("[role='radio'][tabindex='0']")).toHaveLength(1);
expect(target.querySelector(".active-month")?.textContent).toContain("Dec 2026");
expect(target.querySelectorAll(".active-category-value")).toHaveLength(3);
```

Simular `ArrowLeft`, `Home`, `End`, clic y hover; comprobar foco, selección y restauración. Reemplazar props mediante `createClassComponent` para demostrar que reorder/shrink conserva la clave o cae al último mes. Verificar que cada radio anuncia mes y las tres cantidades, que el modo tabla tiene un único caption y que hay tres patrones de línea diferentes.

- [ ] **Step 2: ejecutar RED**

Run: `pnpm exec vitest run src/lib/components/CategoryLines.test.ts`
Expected: FAIL porque el componente actual expone 12 botones tabbables, usa índices y no tiene resumen estable.

- [ ] **Step 3: implementar selección estable y resumen**

Cambiar `MonthPoint` para incluir `key: string`, sustituir `activeIndex` por:

```ts
let selectedKey = $state(untrack(() => series.at(-1)?.key ?? null));
let hoverKey = $state<string | null>(null);
const selectedIndex = $derived(/* find key; fallback last */);
const hoverIndex = $derived(/* find key; null when stale */);
const visibleIndex = $derived(hoverIndex ?? selectedIndex);
```

Usar `nextChartIndex` para un radiogroup roving idéntico al patrón probado en `MonthlyBars`. El resumen visible contiene mes/año y tres filas con nombre, participación y valor exacto. El hover sólo previsualiza; foco/clic/teclado seleccionan.

- [ ] **Step 4: diferenciar las líneas y hacer responsive el componente**

Usar clases por kind y estilos de trazo:

```css
.series-line.vid { stroke-dasharray: 5 3; }
.series-line.pdf { stroke-dasharray: 2 3; }
```

Añadir `container: category-lines / inline-size`; a menos de 480 px compactar el resumen y ocultar etiquetas alternas con CSS, sin medir `clientWidth`. Los hit targets de los meses cubren toda la columna del gráfico y el foco permanece visible.

- [ ] **Step 5: ejecutar GREEN**

Run: `pnpm exec vitest run src/lib/components/CategoryLines.test.ts src/lib/components/MonthlyBars.test.ts`
Expected: PASS; MonthlyBars no regresa.

### Task 3: StorageDonut escalable y accesible

**Files:**
- Create: `src/lib/components/StorageDonut.test.ts`
- Modify: `src/lib/components/StorageDonut.svelte`

- [ ] **Step 1: escribir pruebas fallidas del dataset visual y completo**

Montar seis formatos y comprobar:

```ts
expect(target.querySelectorAll(".donut-seg")).toHaveLength(5);
expect(target.querySelectorAll(".legend-row")).toHaveLength(5);
expect(target.querySelector(".legend-row:last-child")?.textContent).toContain("Other");
expect(target.querySelectorAll(".visually-hidden tbody tr")).toHaveLength(6);
```

Verificar una única fila tabbable/seleccionada, navegación con flechas/Home/End, hover sin perder selección, porcentaje y tamaño visibles, `aria-label` completo y modo tabla con seis filas originales.

- [ ] **Step 2: ejecutar RED**

Run: `pnpm exec vitest run src/lib/components/StorageDonut.test.ts`
Expected: FAIL porque el componente representa todos los segmentos y todas las filas son tabbables.

- [ ] **Step 3: separar `segments` de `displaySegments`**

Añadir prop `otherLabel` y derivar:

```ts
const displaySegments = $derived(groupDonutDisplaySegments(segments, otherLabel));
const rings = $derived(donutSegments(displaySegments.map((segment) => segment.bytes), opts));
```

Usar `displaySegments` sólo en anillo/leyenda y `segments` en ambas tablas. Mantener el total basado en datos originales y asegurar igualdad con la suma agrupada.

- [ ] **Step 4: convertir la leyenda en selector canónico**

Guardar `selectedKey` y `hoverKey`; derivar índices por clave. Las filas usan `role=radio` dentro de `role=radiogroup`, roving tabindex y `nextChartIndex`. Mostrar nombre, porcentaje y tamaño en texto; aplicar clase activa tanto a fila como a arco.

- [ ] **Step 5: implementar container queries y reduced motion**

Añadir `container: storage-donut / inline-size` en el shell que envuelve el contenido. Diseño amplio de tres columnas; a 379 px o menos, hechos en grid de tres columnas y zona anillo/leyenda en dos; a 320 px o menos, apilar. Añadir:

```css
@media (prefers-reduced-motion: reduce) {
  .donut-seg { transition: none; }
  .donut-seg.active { transform: none; }
}
```

- [ ] **Step 6: ejecutar GREEN**

Run: `pnpm exec vitest run src/lib/components/StorageDonut.test.ts src/lib/charts.test.ts`
Expected: PASS.

### Task 4: Formato internacional del Dashboard

**Files:**
- Modify: `src/lib/utils.ts`
- Modify: `src/lib/utils.test.ts`
- Modify: `src/lib/components/DashboardPage.svelte`

- [ ] **Step 1: escribir pruebas fallidas del formateador localizado**

Definir el nuevo contrato sin cambiar `formatBytes` usado por el resto de la aplicación:

```ts
expect(formatBytesLocalized(2_840_000_000, "en")).toBe("2.84 GB");
expect(formatBytesLocalized(2_840_000_000, "es")).toBe("2,84 GB");
expect(formatBytesLocalized(1_024, "de")).toBe("1 KB");
```

Incluir cero, bytes sin decimales, KB/MB/GB/TB y valores no finitos.

- [ ] **Step 2: ejecutar RED**

Run: `pnpm exec vitest run src/lib/utils.test.ts`
Expected: FAIL porque `formatBytesLocalized` no existe.

- [ ] **Step 3: implementar el formateador mínimo**

Añadir una función que seleccione unidad hasta TB y use `Intl.NumberFormat(locale, { maximumFractionDigits: 2 })`; devolver `0 B` para cero o entradas no finitas/negativas. En `DashboardPage`, derivar:

```ts
function formatDashboardBytes(value: number): string {
  return formatBytesLocalized(value, getLocale());
}

function formatDashboardCount(value: number): string {
  return new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 }).format(value);
}
```

Pasar `formatDashboardBytes` a los tres gráficos y usar `formatDashboardCount` en los hechos del donut.

- [ ] **Step 4: ejecutar GREEN**

Run: `pnpm exec vitest run src/lib/utils.test.ts`
Expected: PASS.

### Task 5: Integración, i18n y layout responsive

**Files:**
- Modify: `src/lib/components/DashboardPage.svelte`
- Modify: `src/lib/i18n/locales/en.ts`
- Modify: `src/lib/i18n/locales/es.ts`
- Modify: `src/lib/i18n/locales/fr.ts`
- Modify: `src/lib/i18n/locales/de.ts`
- Modify: `src/lib/i18n/locales/pt.ts`
- Test: `src/lib/components/CategoryLines.test.ts`
- Test: `src/lib/components/StorageDonut.test.ts`

- [ ] **Step 1: añadir contratos de integración fallidos**

Comprobar por fuente que las tarjetas usan `<h3 id=...>`, `aria-labelledby`, que `CategoryLines` recibe puntos con `key`, y que `StorageDonut` recibe `otherLabel={t("dashboard.donutOther")}`. Verificar también que `hasDonutData` exige al menos un segmento y un total positivo, y que una única región viva `polite` y atómica anuncia el fin del escaneo sin emitir contenido antes de completarse. Añadir una prueba de paridad implícita ejecutando `locale.test.ts` tras introducir la clave inglesa.

- [ ] **Step 2: ejecutar RED**

Run: `pnpm exec vitest run src/lib/components/CategoryLines.test.ts src/lib/components/StorageDonut.test.ts src/lib/stores/locale.test.ts`
Expected: FAIL por semántica/prop/key i18n ausentes.

- [ ] **Step 3: integrar títulos y traducciones**

Cambiar spans de título por `h3` y asociar cada `section`. Añadir `dashboard.donutOther` con traducciones humanas:

```ts
// en
donutOther: "Other",
// es
donutOther: "Otros",
// fr
donutOther: "Autres",
// de
donutOther: "Andere",
// pt-BR
donutOther: "Outros",
```

Pasar la nueva prop a `StorageDonut` y mantener `donutData` completo, sin agruparlo en la página.

- [ ] **Step 4: implantar el responsive del Dashboard**

Declarar `container: dashboard / inline-size`, eliminar el scroll anidado del `.dashboard` si el shell ya es propietario y añadir container queries:

```css
@container dashboard (max-width: 820px) {
  .row-a,
  .row-b { grid-template-columns: 1fr; }
  .dash-head { align-items: flex-start; flex-wrap: wrap; }
  .dash-head-actions { flex-wrap: wrap; }
}
```

Ajustar alturas a `min-height` y permitir crecimiento natural para que 900×600 use el scroll del contenido sin cortar tarjetas. Permitir que las etiquetas KPI largas envuelvan sin elipsis. Todos los corner toggles quedan al menos en 36×36 y con `:focus-visible` compartido.

- [ ] **Step 5: ejecutar GREEN y checks focales**

Run: `pnpm exec vitest run src/lib/components/CategoryLines.test.ts src/lib/components/StorageDonut.test.ts src/lib/stores/locale.test.ts`
Expected: PASS.

### Task 6: Revisión visual, accesibilidad y calidad

**Files:**
- Modify only files from Tasks 1–5 when a verified finding requires it.

- [ ] **Step 1: ejecutar suite frontend completa**

Run: `pnpm test`
Expected: todos los tests pasan; registrar el nuevo total exacto.

- [ ] **Step 2: ejecutar typecheck y build**

Run: `pnpm check && pnpm build`
Expected: 0 errores/0 warnings y build terminado.

- [ ] **Step 3: revisar en navegador a dos tamaños y dos temas**

Con el servidor local, inspeccionar 1250×820 y 900×600 en oscuro y claro. Confirmar: sin scroll horizontal, un único scroll vertical, textos sin colisión, tarjetas legibles y tablas contenidas.

- [ ] **Step 4: ejecutar revisiones especializadas**

Solicitar revisión independiente a `revault-ui-engineer`, `revault-svelte`, `accessibility-reviewer` y `revault-devops-qa`. Corregir sólo hallazgos reproducibles y volver a ejecutar los checks afectados.

- [ ] **Step 5: verificación final del diff**

Run: `git diff --check && git status --short && git diff --stat`
Expected: sin whitespace errors; sólo archivos de esta funcionalidad y sus documentos.

El commit, push y PR quedan fuera de este plan hasta que el usuario los solicite explícitamente.

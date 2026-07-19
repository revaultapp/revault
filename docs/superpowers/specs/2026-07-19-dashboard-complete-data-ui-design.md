# Dashboard completo con datos reales — especificación de diseño

**Fecha:** 2026-07-19
**Rama:** `codex/dashboard-complete-data-ui`
**Base:** `main` en `301b378` (`feat(dashboard): refine monthly savings chart`)

## Objetivo

Completar el Dashboard aprobado visualmente para que una persona entienda de inmediato cuánto ha ahorrado ReVault, cómo se reparte ese ahorro y qué contenía su último escaneo. Debe conservar la apariencia actual: sobria, limpia, compacta y coherente en claro y oscuro, desde la ventana predeterminada de 1250×820 hasta el mínimo soportado de 900×600.

La implementación mostrará exclusivamente datos procedentes de los stores reales. No se añadirán valores de demostración, semillas ni fallbacks ficticios al producto.

## Alcance

- Conservar la cabecera, los cuatro KPI y el gráfico mensual ya integrado.
- Refinar `CategoryLines` para convertir “Ahorro por tipo de archivo” en una lectura inmediata de tendencia y reparto.
- Refinar `StorageDonut` para que “Último escaneo” soporte desde uno hasta muchos formatos sin desbordarse.
- Hacer responsivo el conjunto del Dashboard mediante el ancho real disponible en su contenedor.
- Mantener los modos gráfico/tabla, los estados vacío, escaneando y error, y las cinco traducciones actuales.
- Añadir pruebas del comportamiento y de las transformaciones de datos que evitan regresiones con datos crecientes.

## Jerarquía de información

1. Los KPI responden “qué ha conseguido ReVault”.
2. El ahorro mensual responde “cómo está evolucionando”.
3. El ahorro por tipo responde “qué clase de archivo produce ese ahorro”.
4. El último escaneo responde “qué se analizó más recientemente”.

No se añaden nuevas tarjetas ni controles globales. La acción primaria sigue siendo “Escanear carpeta”, usando la única excepción de botón con gradiente permitida por el sistema.

## Ahorro por tipo de archivo

### Datos

- Fuente: `monthlySeries` y `categoryShares` del store `history`.
- Ventana: los 12 meses que ya entrega el store.
- Series fijas: imágenes, vídeo y PDF.
- Valores exactos: bytes ahorrados por categoría y mes.
- Participaciones: porcentaje de cada categoría sobre el ahorro total de esos 12 meses.

### Presentación

- Cabecera compacta con las tres categorías, su participación y su nombre.
- Gráfico de tres líneas para comunicar tendencia temporal.
- Cada línea se diferencia por color y por patrón de trazo, de modo que el significado no dependa sólo del color.
- El mes seleccionado se resume de forma estable con sus tres valores exactos. El puntero puede previsualizar otro mes sin perder la selección.
- En anchos reducidos se ocultan etiquetas alternas del eje, pero nunca datos.
- El modo tabla conserva los 12 meses y las tres series completas.

### Interacción y accesibilidad

- Un único grupo de meses navegable mediante `ArrowLeft`, `ArrowRight`, `Home` y `End`.
- Sólo un mes forma parte del orden de tabulación; `Enter`, espacio o clic lo seleccionan.
- El estado se identifica por la clave estable del mes, no por un índice susceptible de quedar obsoleto.
- El grupo expone por mes las tres cantidades localizadas; el gráfico incluye resumen accesible y tabla alternativa.
- El título visible de la tarjeta será un encabezado semántico asociado con su contenido.

## Último escaneo

### Datos

- Fuente preferente: `storage.scanResult` de la sesión actual.
- Fallback real: `history.lastScan`, persistido tras un escaneo anterior.
- El total del anillo se calcula a partir de los segmentos que representa; los hechos laterales muestran archivos, número de formatos y formato principal.
- El nombre de carpeta sólo aparece cuando la sesión actual lo conoce; la fecha procede del último escaneo persistido.

### Escalado de formatos

- Los datos se ordenan por bytes descendentes.
- El gráfico y la leyenda muestran como máximo cuatro formatos principales más un segmento localizado “Otros”.
- “Otros” suma bytes y recuentos de todos los formatos restantes.
- Si hay cinco formatos o menos, no se crea “Otros”.
- La tabla visible y la tabla accesible contienen siempre los formatos originales completos; la agrupación es sólo una simplificación visual y no elimina información.
- Las claves de interacción se basan en etiquetas normalizadas, no en posiciones.

### Presentación e interacción

- Composición amplia: hechos a la izquierda, anillo en el centro y leyenda a la derecha.
- La leyenda es el control canónico, con filas suficientemente grandes, porcentaje y tamaño visibles.
- El anillo es una representación visual no interactiva; al enfocar, pasar o activar una fila de leyenda se destaca su segmento y se actualiza el resumen.
- El foco es visible y el estado activo también se expresa mediante texto/peso, no sólo color.
- La tabla muestra tipo, tamaño y número de archivos para todos los formatos.

## Estados

- **Sin historial:** conservar mensaje útil y CTA hacia Optimizar en ambos gráficos de ahorro.
- **Sin escaneo:** icono, explicación breve y CTA para escanear carpeta.
- **Escaneando:** indicador con `role=status`, texto localizado y ruta truncada de forma segura.
- **Error:** `role=alert`, mensaje y acción de reintento.
- **Escaneo completado:** una única región viva `polite` y atómica permanece montada, silenciosa hasta finalizar, y anuncia el total localizado o que no se encontraron archivos compatibles.
- **Con datos:** gráfico por defecto y conmutador a tabla.
- **Tabla:** contenido interno desplazable sin ensanchar la página.

## Responsive

El Dashboard se convierte en contenedor de tamaño. Los cambios dependen del ancho útil del contenido, no únicamente del viewport.

- **Amplio (aprox. ≥900 px útiles):** fila superior en dos columnas; fila inferior `1.25fr / 1fr`; donut en tres zonas.
- **Medio/estrecho (≤820 px útiles, incluido 900×600 con sidebar):** ambas filas se apilan en el orden KPI → mensual → categorías → último escaneo; KPI permanece en dos columnas, sus etiquetas largas pueden envolver, la cabecera permite envolver acciones y las tarjetas tienen altura mínima suficiente.
- **Tarjeta de donut estrecha:** a 379 px o menos, hechos en una franja superior y anillo/leyenda debajo; a 320 px o menos se apilan, siempre sin scroll horizontal.
- Habrá un único propietario del scroll vertical del contenido del Dashboard; tablas pueden tener scroll interno sólo en modo tabla.
- Textos largos de traducciones o rutas pueden envolver o truncarse con acceso al valor completo cuando sea necesario; nunca fuerzan el ancho.

## Sistema visual

- Usar únicamente tokens existentes de `src/app.css`; no se introducen colores hexadecimales en componentes.
- Mantener Plus Jakarta Sans, radios, bordes y sombras existentes.
- No añadir gradientes fuera del CTA de escaneo ya autorizado.
- Duraciones mediante tokens; `prefers-reduced-motion` desactiva la rotación o escalado no esencial.
- Números con cifras tabulares y formato localizado mediante `Intl.NumberFormat`: `formatBytesLocalized` mantiene este contrato separado del formateador general, y separadores, decimales, porcentajes y conteos respetan EN/ES/DE/FR/PT.

## Pruebas y aceptación

- Transformación de segmentos: 0, 1, 5 y más de 5 formatos; orden, suma de “Otros” y conservación del dataset completo para tabla.
- CategoryLines: selección inicial, navegación por teclado, estabilidad ante reorder/shrink y etiquetas accesibles con las tres series.
- StorageDonut: un único control tabbable cuando proceda, interacción teclado/puntero, tabla completa frente a leyenda resumida.
- Dashboard: títulos semánticos, estados y conmutadores con etiquetas correctas.
- `pnpm test`, `pnpm check`, `pnpm build` y `git diff --check` deben pasar.
- Revisión visual a 1250×820 y 900×600, en claro y oscuro, sin desbordamiento horizontal ni contenido cortado.

## Fuera de alcance

- Cambiar el modelo persistido de `history`.
- Añadir rangos temporales, exportación o compartir; los controles deshabilitados no se activan aquí.
- Crear estadísticas acumuladas que el producto todavía no registra.
- Cambiar navegación, sidebar, tipografía o paleta global.
- Incluir datos de demostración en la aplicación.

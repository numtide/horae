# Auditoría de diseño de la interfaz

Estudio de solo lectura sobre `master` `3b08dd25` (rama `study/design-audit`), del 18 de noviembre de 2026.

## Resumen

La interfaz no está rota: ninguna pantalla desborda horizontalmente a 360, 768 o 1280 px, no aparecen
botones sin nombre accesible y casi todo el CSS usa tokens. Las incongruencias vienen de que cada spec
resolvió por su cuenta los mismos patrones. Las pantallas rehechas con el handoff nuevo (Clients, Client
detail, Projects, Project detail, New/Edit project) conviven con las anteriores (Timesheet, Approvals,
Reports, Invoices, Tasks, People, Settings, Importers), y se nota a primera vista:

- **Cuatro tamaños de título de página** (24/500, 24/600, 34/600 y 32/700).
- **Cuatro anatomías de modal**, y cuatro de ellos sin relleno.
- **Tres variantes de `.empty-state`** y una decena de vacíos hechos con texto suelto.
- **Unos 45 textos de carga** con cinco estilos distintos. No hay *skeleton* ni estado de error con reintento.
- **Cinco maneras de elegir un valor.** Menú propio, `Combobox`, `SelectField`, `Select` y `<select>`
  nativo, a veces en la misma clase de filtro.
- **Tres formatos de duración** (`13:00`, `7.00` y `13h`) y dos de fecha (`16 Nov 2026` y `11/01/2026`).
- **Tablas anchas sin alternativa en móvil.** Ya a 1280 px la columna *Total* del timesheet y *Budget
  remaining* de Projects quedan fuera de la vista.
- **Contraste insuficiente en los textos más tenues.** Cabeceras de tabla 2,98:1, *muted* en claro
  2,6:1 y *placeholders* 2,55:1 (1,62:1 en claro).

La mayoría se arregla patrón a patrón con un componente o una clase compartida y una pasada por las
pantallas. La propuesta de PRs está al final. Hay diez decisiones que te corresponden a ti (sección
*Preguntas*). Donde dos estilos conviven no he elegido ninguno.

### Cómo se ha hecho

- **Referencias, leídas enteras:** `DESIGN.md`, `horae.css`, `horae-utils.css` (solo por encima),
  `design/README.md`, `design/IMPLEMENTATION-GAPS.md`, los 32 `*.dc.html` (las líneas de más de 2 000
  caracteres de `Components.dc.html`, partidas para que no se cortara ninguna) y el skill `design-implement`.
- **Código:** búsquedas con `rg` por patrones en `crates/horae/src/{pages,components}` (unas 25 000
  líneas), más lectura de los ficheros implicados.
- **Capturas:** 156 PNG en `audit/screenshots/`, `<pantalla>-<ancho>-<tema>.png`, con 26 pantallas a
  360/768/1280 px en oscuro y claro, más el login. Salen de un servidor aislado, migrado y con la *seed*
  de demo, levantado como `run-design-checks.sh`. El script de captura también mide desbordes, títulos,
  controles sin nombre o sin etiqueta, radios y tamaños de letra. Los resultados están en
  `.data/audit/survey.json` (no versionado).
- **Contrastes:** calculados con la fórmula de WCAG sobre los valores de los tokens.

## Tabla de patrones por impacto

El impacto es el número de pantallas afectadas por lo visible que es (A = se ve en cuanto entras,
M = al usar la pantalla, B = en casos concretos o en el código).

| # | Patrón | Pantallas | Visibilidad | PR |
|---|---|---|---|---|
| 1 | Cabecera de página: título, volver y acciones | 26 | A | PR-2 |
| 2 | Estados vacío, carga y error | ~20 | A | PR-4 |
| 3 | Tablas anchas y móvil | 9 | A | PR-7 |
| 4 | Filtros y selectores de valor | 8 | A | PR-5 |
| 5 | Formato de duración y fecha | 7 | A | PR-6 |
| 6 | Contraste y foco | todas | M | PR-1 |
| 7 | Modales y confirmaciones | 12 diálogos | M | PR-3 |
| 8 | Botones: variantes, tamaños y acciones de fila | ~15 | M | PR-8 |
| 9 | Avisos de éxito | 8 | M | PR-9 |
| 10 | Textos y tono | ~12 | M | PR-10 |
| 11 | Valores fuera de tokens | 5 | B (la mitad se ve en tema claro) | PR-1 |
| 12 | Selector de periodo | 4 | M | PR-6 |
| 13 | Pantallas sin prototipo (D-013) | 9 | M | consecuencia de PR-2 a PR-5 |
| 14 | Documentación desfasada | — | B | PR-0 |

______________________________________________________________________

### 1. Cabecera de página (título, volver y acciones)

**Qué está mal.** El título de página tiene cuatro aspectos, según la spec que tocó la pantalla
(medido en el navegador a 1280 px):

| Aspecto | Pantallas |
|---|---|
| `.page-title`, 24 px / 500, `--color-text` | Timesheet, Approvals, Reports, Invoices, Invoice settings, People, Workspace settings, Importers, Harvest, CSV, Runs, Settings, Components |
| `page-title text-4xl font-semibold text-strong tracking-tight`, 34 px / 600 | Clients, Client detail, Projects, Project detail |
| `text-4xl font-semibold text-strong`, sin `.page-title` ni `tracking-tight` | New project, Edit project |
| `text-2xl font-semibold`, 24 px / 600 | Tasks, Audit log |
| `h1` del 404, 32 px / 700 | 404 |

Los 14 prototipos usan uno solo: Newsreader 34/600, −0,015 em, `#F6F2E9`. El «volver» también varía:

- `btn btn-ghost` alineado al texto en Project detail y New project, y con sangría en Client detail.
- `btn btn-secondary` arriba a la derecha en Invoice settings.
- Enlace de texto «← Back to Timesheet» en todo el área de administración.
- En Harvest conviven ese enlace y una miga de pan (`Importers / Harvest`).

Las acciones de cabecera son de tamaño normal en casi todas las pantallas y `btn-sm` en Tasks. En
Reports, a 360 px, se alinean a la derecha, y en las demás pantallas a la izquierda.

**Reglas que incumple.**

- DESIGN.md, *Type scale*: `4xl` es «refreshed handoff page heading».
- Kit, *Scale & tokens*: «4xl — page titles · display only».
- Kit, *Back link*: un solo enlace en su fila, con flecha de 16 px y texto de 14 px, que «replaces a
  breadcrumb; never both».

**Ficheros.**

- `pages/{timesheet,approvals,reports,reports/scoped,invoices,invoices/settings,admin/people,admin/workspace,importers,importers/harvest/*,settings,gallery}.rs`, con `class: "page-title"`.
- `pages/{tasks,permission_audit}.rs`, con `text-2xl`.
- `pages/new_project.rs:149,232,257,648`.
- `pages/projects.rs:483,486`.
- `pages/invoices.rs:140`.
- `pages/invoices/settings.rs:179`.
- `components/admin_shell.rs`.
- `.page-title` en `horae.css:1056`.

**Capturas.** `approvals-1280-dark.png` frente a `clients-1280-dark.png`, `tasks-1280-dark.png`,
`invoice-settings-1280-dark.png`, `importer-harvest-1280-dark.png` y `client-detail-1280-dark.png`
frente a `project-detail-1280-dark.png` (sangría del volver).

**Propuesta.** Un componente `PageHeader { title, eyebrow?, back?, actions }` y una sola definición de
`.page-title`, con el tamaño que decidas (pregunta 1). El «volver» pasa a ser una clase `.back-link`
según el kit, y se quita la miga de pan de Harvest o el enlace del shell, uno de los dos. Las acciones
de cabecera van siempre a tamaño normal y se envuelven igual en todas las pantallas.

### 2. Estados vacío, carga y error

**Qué está mal.**

- **Vacío.** Hay tres variantes de `.empty-state`:

  - Projects: `bg-secondary rounded-xl` con tesela de icono.
  - Clients: `rounded-xl`, tesela y sin fondo propio.
  - Timesheet y gallery: la base, con un emoji 🗓 como icono.

  En el resto, el vacío es texto suelto:

  - Approvals: `card p-8 text-center` + «No approvals found.».
  - Invoices: `text-muted text-sm p-5`.
  - Reports y Detailed time: `p role=status`.
  - El día del Timesheet: `ts-day-empty`.
  - Client detail (proyectos y facturas) y Project detail (`<ul>` sin estilo).
  - Historial de Harvest.

  Casi ninguno dice qué hacer a continuación.

- **Carga.** Hay unos 45 `p { "Loading…" }` repartidos con cinco combinaciones de clase:
  `text-muted text-sm`, `text-sm text-secondary`, `text-secondary`, sin clase o dentro de `.modal-body`.
  `aria-busy` aparece en unos y falta en otros. No existe el *skeleton* del handoff (`Skeleton.dc.html`).

- **Error.** `alert alert-danger` se usa con coherencia (unas 70 veces), pero no hay ningún estado de
  error a página completa con «Try again» (`ErrorState.dc.html`), y `.banner-danger`, que DESIGN.md
  reserva para errores con acción, solo aparece en la galería. El botón de reintento se llama «Retry»
  en Projects, Timesheet y Reports, y «Try again» en Harvest.

**Reglas que incumple.**

- DESIGN.md, *Screen States*: el vacío es «`.empty-state` with copy that says what to do next».
- Las cargas deben ser «inline, per region… keep the layout stable».
- Los errores que piden acción van en «`.banner-danger`».
- Kit, *Empty state* y *Error state*: misma silueta, disco de 52 px, título serif y una acción.

**Ficheros.** `pages/approvals.rs:242`, `invoices.rs:149`, `timesheet.rs:1877,2188`,
`clients/detail.rs:240,308`, `projects.rs:910-935,1463,1468`, `reports.rs:387,463`,
`reports/scoped*.rs` y `importers/harvest/overview.rs:227`. Los textos de carga, en la lista de
`rg '"Loading'`. Estilos en `horae.css:1082-1104`.

**Capturas.** `approvals-1280-dark.png`, `invoices-1280-dark.png`, `importer-harvest-1280-dark.png`
(«No runs yet…») y `project-detail-1280-dark.png` (listas con viñetas).

**Propuesta.** Tres componentes: `EmptyState { icon, title, body, action?, link? }`,
`ErrorState { kind, title?, body?, on_retry }` y `Loading { label }` (con `role=status` y `aria-busy`
siempre). Un `Skeleton` de tabla queda opcional, para listados. Una sola variante visual de
`.empty-state`: o se adopta la de Projects como base o se borra (pregunta 2, junto al tamaño de título).
«Try again» o «Retry», un solo verbo (pregunta 6).

### 3. Tablas anchas y diseño adaptable

**Qué está mal.**

- **Timesheet, semana.** La rejilla mide como mínimo 1 060 px (`minmax(280px,1fr) repeat(7,92px) 92px 44px`). A 1280 px, con la barra lateral de 264 px, el hueco útil es de unos 950 px, así que las
  columnas *Total* y borrar quedan fuera sin que nada lo indique. A 360 px solo se ven los nombres de
  proyecto y ninguna celda de día.
- **Projects.** A 1280 px la cabecera «BUDGET REMAINI…» queda cortada por el contenedor.
- **A 360 px.** Clients, Projects, People, Tasks, Approvals, Reports e Invoices hacen *scroll*
  horizontal dentro de la tarjeta, sin alternativa apilada ni indicación de que hay más columnas.
  Estado y acciones quedan fuera de la vista.
- **Nombres partidos.** Los nombres se parten a una palabra por línea («Admin / User» en People,
  «Acme / Website / Redesign» en Reports a 360 px).
- **«Timesheet for».** Es un `<select>` a todo el ancho pegado al borde superior de la rejilla, sin
  separación.

**Reglas que incumple.**

- DESIGN.md, *Narrow*: «usable at 390px and at the 768px mobile-navigation breakpoint».
- DESIGN.md, *Layout → Narrow content*: «Dense project/time grids… scroll within their own containers»,
  así que el *scroll* está permitido, pero no la pérdida de columnas a 1280 px.
- DESIGN.md, *Long content*.

**Ficheros.** `horae.css:1564-1568` (`.ts-row`) y `1803-1817` (`.proj-grid`), `pages/timesheet.rs`,
`pages/projects.rs`, `pages/clients.rs`, `pages/admin/people.rs`, `pages/tasks.rs` y
`components/table.rs`.

**Capturas.** `timesheet-week-1280-dark.png`, `timesheet-week-360-dark.png`, `projects-1280-dark.png`,
`projects-360-dark.png`, `clients-360-light.png`, `reports-360-dark.png` y `admin-users-1280-dark.png`.

**Propuesta.**

- Timesheet: columna de proyecto con `minmax(200px,1fr)` y la columna de total fijada a la derecha,
  para que quepa a 1280 px.
- Por debajo de 768 px: un modo de lista por día para el Timesheet, o fila por proyecto con los días
  apilados.
- Tablas de listado: o una regla común de «tarjeta apilada» por debajo de 640 px (nombre, estado,
  acción principal) o un borde de desvanecido que indique el *scroll* (pregunta 8).
- Columna de nombre con un ancho mínimo, como ya hace `.client-table` (`--width-client-name`).

### 4. Filtros y selectores de valor

**Qué está mal.** El mismo gesto, «filtrar la lista», se resuelve de cuatro formas:

- Projects y Clients: `Menu` propio con disparador `btn-secondary`, alineado a la derecha.
- Tasks y People: `<select class="form-select">` nativo a todo el ancho, con etiqueta «Show». En People
  la flecha es la nativa del navegador, distinta de la de `.form-select`.
- Reports: una tarjeta con selects etiquetados en rejilla.
- Approvals: estado con `Segmented`.

Para elegir un valor en formularios conviven `Menu`, `Combobox`, `SelectField`, `form::Select` y
`<select>` sin componente (`reports.rs` ×3, `tasks.rs`, `admin/people.rs`, `admin/add_person.rs`,
`admin/permission_editor.rs`, `components/project_task_picker.rs`). El buscador lleva lupa en Projects
y no la lleva en Clients.

**Reglas que incumple.**

- Kit, *Selects & menus*: «Custom dropdowns (never native `<select>`)».
- Kit, *Filter bar*: «a row of them reads as one instrument».
- Kit, *Search field*: lupa siempre.

**Capturas.** `projects-1280-dark.png` frente a `tasks-1280-dark.png`, `admin-users-1280-dark.png`,
`reports-1280-dark.png`, `approvals-1280-dark.png` y `clients-1280-dark.png` (buscador sin lupa).

**Propuesta.**

- Una `FilterBar` con disparadores de `Menu` para todos los listados.
- Un `SearchField` compartido con lupa.
- Decidir si el `<select>` nativo se permite en formularios sencillos (pregunta 7). Si no, migrar los
  ocho usos a `SelectField`.
- Fusionar `form::Select` y `SelectField`, o documentar cuándo se usa cada uno.

### 5. Formato de duración y de fecha

**Qué está mal.**

- **Duración, tres formatos.** `format_hhmm` da `13:00` en Timesheet, Approvals y Client detail.
  `format_hours2` da `7.00` en Reports e Invoices. En Projects, un formateador propio da `13h` o
  `0.98h` (`pages/projects.rs:32`). El mismo proyecto muestra «13h» en Projects y «13:00» en Client
  detail.
- **Fecha, dos formatos.** La app escribe `16 Nov 2026`, pero los `<input type=date>` nativos (Reports,
  preparar factura, hitos de New project) muestran el formato del navegador: `11/01/2026` en inglés de
  EE. UU.

**Reglas que incumple.** `IMPLEMENTATION-GAPS.md`, *format.js*: «Agree on H:MM vs decimal-hour
display». Está pendiente desde septiembre.

**Capturas.** `projects-1280-dark.png` («13h»), `client-detail-1280-dark.png` («13:00») y
`reports-1280-dark.png` («7.00» y «11/01/2026»).

**Propuesta.** Un solo formateador de duración en `horae-core`, con la convención que elijas
(pregunta 3), y retirar el de `projects.rs`. Sustituir los `type=date` por el `DatePicker` compartido
o, como mínimo, por un campo de texto con el formato de la app.

### 6. Contraste y foco

**Qué está mal.**

| Par | Contraste | Mínimo AA |
|---|---|---|
| `--color-label` (cabeceras de tabla, 12 px) sobre `--color-bg-secondary`, oscuro | **2,98** | 4,5 |
| *Placeholder* (`--color-border-strong`) sobre `--color-bg`, oscuro | **2,55** | 4,5 |
| *Placeholder* sobre `--color-bg`, claro | **1,62** | 4,5 |
| `--color-text-muted` sobre `--color-bg`, claro (textos de carga, pistas) | **2,60** | 4,5 |
| `--color-text-muted` sobre `--color-bg-secondary`, claro | **2,86** | 4,5 |
| `--color-label` sobre `--color-bg-secondary`, claro | 3,50 | 4,5 |
| Acento claro (día de hoy) sobre `--color-bg` | 2,69 | 3 (texto grande) |
| `--color-text-muted` sobre `--color-bg-secondary`, oscuro | 3,88 | 4,5 |

El foco se pinta de tres maneras: `box-shadow: var(--ring)` en botones, anillo interior (*inset*) en
menú, paginador y *collapse*, y `outline: 2px` en `.proj-scroll` y el selector de ficheros. Los enlaces
(nombres en tablas, «View in Projects», «Back to…») no tienen regla y muestran el contorno por defecto
del navegador. El `<select>` de tema de Settings no tiene etiqueta asociada (el sondeo lo marca).

**Reglas que incumple.**

- DESIGN.md, *Accessibility*: «Color contrast meets WCAG AA».
- *Screen States → Keyboard*: «visible `--ring` focus».
- Kit, *Focus*: «Ring never replaced by an outline».
- El kit mismo avisa: «Anything fainter than [text-disabled] fails contrast».

**Ficheros.** Tokens en `horae.css:36-37` (oscuro) y `205-206` (claro), placeholders en `horae.css:942`,
`a {}` en `horae.css:286` y `pages/settings.rs` (select de tema).

**Propuesta.** Subir `--color-label` (oscuro) y `--color-text-muted` (claro) hasta 4,5:1 y usar
`--color-text-muted` en los placeholders. Una regla `a:focus-visible, [tabindex]:focus-visible { box-shadow: var(--ring) }`
y la etiqueta del select de tema. Es un cambio de tokens con poco código: va primero (PR-1).

### 7. Modales y confirmaciones

**Qué está mal.** Cuatro anatomías sobre el mismo `Modal`:

- **Estándar.** `.modal-title` centrado + `.modal-body` + `.modal-actions`, con los botones a todo el
  ancho y la acción principal primero. Lo usan Projects (archivar, exportar), el archivo de cliente, los
  descartes de New project y las tareas.
- **Duplicado del timesheet.** `.ts-modal-title`, `.ts-modal-body` y `.ts-modal-actions`, que copian
  `.modal-*` con literales (`font-size: 18px`).
- **Título a la izquierda.** El editor de cliente, con `modal-title text-left px-6 py-4`, y el de
  permisos, con `text-2xl` alineado a la izquierda.
- **Sin anatomía.** Un `h2` sin clase y párrafos sin relleno, pegados al borde del panel: desconectar
  Harvest (`connection_card.rs:165`), cambiar de cuenta (`change_account.rs:60`) y el exceso de
  facturación, en edición y en preparación (`invoices/editing.rs:253`, `invoices/preparation.rs:386`).
  En estos cuatro, además, los botones van a la derecha (`justify-end`) con «Cancel» primero, o a la
  izquierda con la acción peligrosa primero.

Confirmar algo destructivo usa `btn-danger` (borde rojo) o `btn-primary` (archivar cliente o
proyecto), sin regla. No existe la variante de peligro sólida del handoff (`hb-danger-solid` en
`ConfirmDialog.dc.html`).

**Reglas que incumple.**

- DESIGN.md: «Build each state from the existing classes».
- Kit, *Modal*: «One anatomy for every dialog… a centred serif title bar… a footer whose primary action
  sits right».
- `ConfirmDialog.dc.html`: un único componente para acciones destructivas, con icono, título serif y la
  confirmación primero. El handoff se contradice: el kit pone la principal a la derecha y casi todas las
  pantallas, a la izquierda (pregunta 4).

**Capturas.** No se pueden abrir con la *seed*, porque piden Harvest conectado o excesos de factura. La
evidencia está en el código citado.

**Propuesta.** Un `ConfirmDialog { title, body, confirm_label, tone, busy, on_confirm, on_dismiss }`
sobre `Modal`, y slots fijos (`ModalHeader`, `ModalBody`, `ModalFooter`) para el resto. Borrar
`.ts-modal-*`. Decidir el orden y la alineación del pie, y si el peligro es borde o relleno
(preguntas 4 y 9).

### 8. Botones: variantes, tamaños y acciones de fila

**Qué está mal.**

- **El componente apenas se usa.** `components/button.rs` aparece 12 veces, frente a 267 `class: "btn …"`
  escritos a mano.

- **El envío de formularios no sigue la regla.** DESIGN.md quiere `btn-solid` para «form submits»,
  pero solo lo usan Invoice settings y la aprobación en Approvals. Settings, clientes, tareas, personas
  y New project envían con `btn-primary`.

- **Botones propios del timesheet.** `ts-add`, `ts-addrow`, `ts-day-action`, `ts-del` y `ts-project-icon`
  repiten `.btn-icon` o `.btn-secondary btn-sm` con medidas propias (`border-radius: 9px`, `padding: 10px`).

- **Acciones de fila.**

  - Clients: botones sueltos «Edit» y «Archive».
  - People: «Rates», «Permissions» y «Archive», que se parten en dos líneas a 1280 px.
  - Tasks: «Edit».
  - Project detail: menú «Actions».

  El handoff usa siempre un menú compacto «Actions» por fila.

- **El estado no siempre es una insignia.** En Tasks el estado es texto plano («Active»), y en Clients
  y People es una insignia (`badge`).

- **Archivar se ofrece de dos maneras.** Botón en la cabecera de Client detail («Archive client») y
  opción del menú en Project detail.

**Reglas que incumple.**

- DESIGN.md, *Buttons*: Solid para «form submits».
- Kit, *Control heights*: «Interactive height never falls off these four».
- Kit, *Dropdown*: «compact for table rows».

**Capturas.** `clients-1280-dark.png`, `admin-users-1280-dark.png`, `tasks-1280-dark.png`,
`invoice-settings-1280-dark.png` y `client-detail-1280-dark.png` frente a `project-detail-1280-dark.png`.

**Propuesta.** Decidir la regla primaria/sólida (pregunta 10) y aplicarla. Usar `Button` o, como mínimo,
eliminar las clases propias del timesheet que copian `.btn`. Un patrón `RowActions` (menú compacto) para
todas las tablas, y el estado siempre con `Badge`.

### 9. Avisos de éxito

**Qué está mal.** Guardar o archivar se confirma de cuatro maneras:

- `Toast`, abajo a la derecha: Projects, Clients y Client detail.
- `alert alert-success` dentro de la página: también Projects (`projects.rs:545,559,849`), Approvals y
  Missing people.
- Texto en línea: «Saved.» en Invoice settings, «Profile saved.» en verde (`text-success`) en Settings,
  y un párrafo gris con `role=status` en Tasks («Task archived.») y People («Permissions saved.»,
  rates).

El `Toast` de la app va abajo a la derecha, con borde de color a la izquierda. El del handoff va abajo
al centro, en negro, con icono y una acción opcional («Undo»).

**Reglas que incumple.** Kit, *Form dirty state*: «Saving confirms with a toast, never silence».
`Toast.dc.html`: «Bottom-centre notice».

**Propuesta.** El éxito de una mutación va siempre en `Toast`, a través de un contexto de avisos de
página (`ToastContainer` en `AppLayout`), y `.alert-success` queda para estados persistentes. Decidir la
posición y el estilo (pregunta 5).

### 10. Textos y tono

**Qué está mal.**

- **Lo contrario de archivar.** «Reactivate» en proyectos y «Restore» en clientes, personas y tareas.
- **Personas.** En la barra lateral y la subnavegación se llaman «Team», y la página se titula «People».
  La subnavegación dice «Settings» y la página, «Workspace settings».
- **Mayúsculas.** «New Invoice» frente a «New project», «New client» y «New task».
- **Reintentar.** «Retry», «Retry report», «Retry tags», «Retry people», etc., frente a «Try again».
- **Puntos suspensivos.** «Archive task…» y «Restore task…» los llevan, y el resto de acciones que abren
  diálogo no.
- **Volver.** Desde la administración se vuelve a «Timesheet», no a la pantalla de la que vienes.
- **Acciones que dicen lo mismo con palabras distintas.** «Cancel changes» y «Cancel over-invoicing»
  frente a «Cancel». «Close» frente a «Dismiss».
- **Vacíos con terminación distinta.** «No approvals found.» y «No invoices yet. Generate one…» llevan
  punto, y los títulos de `.empty-state` no.

**Reglas que incumple.** DESIGN.md, *Empty*: «copy that says what to do next». El skill `humanizer`
recomienda un vocabulario coherente.

**Propuesta.** Un glosario corto en DESIGN.md (verbos, nombres de secciones y mayúsculas tipo
*sentence case*) y un PR que lo aplique (preguntas 6 y 11).

### 11. Valores fuera de los tokens

**Qué está mal.**

- **Color literal en `horae.css`.**

  - Calendario: `#2b7a66`, `#e6b45f`, `#f6f2e9`, `#4a3208`, `#8fc4b3` y `#6b4a10` en
    `.ts-cal-event` y `.ts-cal-ev-*`.
  - `.ts-submit-caret`: `#164034` y `#0f3d33`.
  - Sombras con `rgba` en `.auth-card`, `.ts-submit` y `.btn-split`.

  En tema claro, el texto de la tarjeta de hoy pierde contraste. El comentario de la línea 1714
  («Design-exact darks, not tokens») contradice al de la siguiente, que dice lo contrario.

- **Tamaños de letra fuera de escala.** 10,5 px (rótulos de la barra lateral, 102 nodos), 11, 11,5,
  12,5, 13 y 14,5 px, sobre todo en `.ts-*`. El kit: «no 13.5px type. The scale skips 7, 9, 11 and 13–15».

- **Radios literales.** `11px` (barra lateral plegada), `9px` (`.ts-addrow`) y `6px` (`.ts-cal-event`),
  en vez de `--radius-*`.

- **`style:` estáticos en `rsx!`.** Están en `pages/invoices.rs:233,276,290` (`margin-left: 0.5rem`, una
  rejilla) y en `PRE_LINE` de `invoices/settings.rs`. Los del calendario y del ancho de la barra lateral
  son dinámicos y son correctos.

- **CSS duplicado.** `.alert-warning` está definido dos veces (`horae.css:1115` y `1901`), y
  `.ts-modal-*` duplica `.modal-*` (patrón 7).

- **La página de login** (`crates/api/src/auth/page.rs`) copia la paleta oscura a mano y no responde al
  tema claro (`login-1280-light.png` sale oscuro).

**Reglas que incumple.** DESIGN.md, *Tokens & utilities*: «never hardcode a colour, spacing, or radius».
Skill `design-implement`: «No inline `style=`, no hardcoded colors».

**Propuesta.** Tokens para los colores del evento de calendario en los dos temas, mover los tamaños a la
escala, borrar los duplicados y las utilidades en línea. La página de login, a decidir: o carga
`horae.css` o se documenta que es solo oscura.

### 12. Selector de periodo

**Qué está mal.**

- **Timesheet.** `ts-pager` con flechas ←/→ en texto, icono ▦ y `DatePicker`.
- **Approvals.** Reutiliza las clases `ts-pager-*` con enlaces y sin icono ni calendario.
- **Reports e Invoice preparation.** Usan dos campos `type=date` nativos.
- **Project detail.** No tiene periodo, y el prototipo sí.

El handoff tiene un único `PeriodNav` (con `caret-left/right` y calendario), «Used in Timesheet,
Approvals, Reports and Project Detail».

**Capturas.** `timesheet-week-1280-dark.png`, `approvals-1280-dark.png` y `reports-1280-dark.png`.

**Propuesta.** Extraer `PeriodNav` del Timesheet a `components/` (`IMPLEMENTATION-GAPS.md` ya lo
condicionaba a un segundo consumidor, y Approvals lo es) y usarlo en Approvals. Para Reports hace falta
un modo de rango personalizado: es trabajo de producto, no solo visual.

### 13. Pantallas sin prototipo (D-013)

D-013 (`specs/harvest-parity.md:101`) permite componer las pantallas sin maqueta «from current Horae
components and tokens». Estado actual:

| Pantalla | ¿Componentes actuales? | Desvíos |
|---|---|---|
| Invoices (lista) | sí | título antiguo; vacío en texto; «New Invoice» |
| Invoice detail | sí | `style:` estáticos (`invoices.rs:233,276,290`) |
| Invoice settings | sí | volver como botón arriba a la derecha; «Saved.» en línea; envío `btn-solid` |
| Tasks | en parte | título `text-2xl`; filtro nativo a todo el ancho; botones `btn-sm`; estado sin insignia |
| Audit log | en parte | título `text-2xl` |
| People, rates, permissions | en parte | filtro nativo; acciones de fila que se parten; modal de permisos con su propio título |
| Settings | en parte | existe prototipo (`08_Settings`), pero la página no usa la subnavegación de perfil: tarjetas apiladas con lista de 50 viñetas |
| Harvest runs y run view | sí | volver y miga de pan a la vez; vacíos en texto |
| 404 | no | tarjeta propia fuera del shell, sin navegación; `h1` 32/700 |

Ninguna necesita un prototipo nuevo: se arreglan solas con PR-2 a PR-5.

### 14. Documentación desfasada

- **DESIGN.md.**
  - La tabla *Pages* no se corresponde con las rutas actuales: cita `/auth/register`, `Dashboard`,
    `/time` y `AdminUsers`, y le faltan Tasks, Approvals, Reports, Audit e Importers.
  - Cita `design/project/dark.dc.html`, que no existe.
  - Dice que el punto pine del ítem activo va «leading», cuando el CSS lo pone detrás.
- **`horae.css`.** El comentario de `--radius-lg` dice «cards, tables», pero las tablas usan
  `--radius-panel`.
- **`design/IMPLEMENTATION-GAPS.md`.** Sigue diciendo que Client detail «only renders `Client detail for {id}`» y que el panel de proyecto es un marcador de posición. Ambos existen ya.

**Propuesta.** PR-0, solo de documentación, que actualice esas tablas y enlace esta auditoría.

______________________________________________________________________

## Propuesta de PRs

Cada PR arregla un patrón en todas las pantallas a la vez y lleva sus capturas de antes y después con el
mismo runner. Tamaños aproximados: S < 150 líneas, M 150–500, L > 500.

| Orden | PR | Patrones | Tamaño | Depende de | En paralelo con |
|---|---|---|---|---|---|
| 0 | Docs: DESIGN.md (páginas, enlaces) e IMPLEMENTATION-GAPS al día | 14 | S | — | todo |
| 1 | Tokens: contraste de label, muted y placeholder; foco de enlaces; colores del calendario; tamaños fuera de escala; duplicados de CSS | 6, 11 | M | — | 0, 3, 6 |
| 2 | `PageHeader` + `.back-link`: un solo título y un solo volver en las 26 pantallas | 1, 13 | M | pregunta 1 | 3, 6 |
| 3 | `ConfirmDialog` + slots de `Modal`; borrar `.ts-modal-*`; arreglar los 4 modales sin relleno | 7 | M | preguntas 4 y 9 | 1, 2, 6 |
| 4 | `EmptyState`, `ErrorState` y `Loading` en las ~20 pantallas | 2, 13 | L (mecánico) | 2, por el ritmo vertical | 5 |
| 5 | `FilterBar`, `SearchField` y migración de los `<select>` nativos | 4 | M | pregunta 7 | 4 |
| 6 | Un solo formateador de duración; `PeriodNav` compartido en Approvals; fuera los `type=date` donde haya `DatePicker` | 5, 12 | M | pregunta 3 | 1, 2, 3 |
| 7 | Tablas: timesheet que quepa a 1280 px, modo apilado o indicador de *scroll* por debajo de 640 px, columna de nombre mínima | 3 | L | 5 (filtros arriba) y pregunta 8 | 8 |
| 8 | `RowActions` en tablas, `Badge` para el estado, regla primaria/sólida y fuera las clases `ts-*` que copian `.btn` | 8 | M | pregunta 10 | 7 |
| 9 | Avisos de éxito unificados en `Toast` desde `AppLayout` | 9 | S–M | pregunta 5 | 7, 8 |
| 10 | Glosario de textos en DESIGN.md y pasada de textos | 10 | S | preguntas 6 y 11 | cualquiera; mejor al final para no chocar |

**Para empezar ya:** PR-0 y PR-1 no dependen de ninguna pregunta y quitan los problemas de
accesibilidad. Después, PR-2 y PR-3 en paralelo, porque cambian lo que más se ve.

**Riesgo común.** Las suites de `crates/horae/tests/browser/*.cjs` (sobre todo `shared-style-audit.cjs`,
`responsive-layout.cjs` y `modals.cjs`) comprueban clases y medidas. Cada PR debe grabar la línea base
de `shared-style-audit` en el commit de partida y compararla, como piden las specs.

## Preguntas para ti

1. **Título de página.** ¿Newsreader 34/600 (todos los prototipos y las pantallas nuevas) en todas las
   pantallas, o 24/500 (la mayoría de las pantallas actuales)? DESIGN.md hoy lo deja como opción por
   pantalla.
1. **Estado vacío.** ¿La versión de Projects (tarjeta `bg-secondary`, radio 16 y tesela de icono) pasa a
   ser la base de `.empty-state`, o se mantiene la base actual (hundida, radio 11, disco de 46 px)?
1. **Duración.** ¿`H:MM` en todas partes, decimal (`7.00`) en todas partes, o `H:MM` en la entrada de
   tiempo y decimal en informes y facturas? ¿O una preferencia por usuario, como hace Harvest?
1. **Pie de diálogo.** ¿Acción principal a la derecha con «Cancel» a la izquierda (kit, *Modal*) o
   principal primero a la izquierda (ConfirmDialog y la mayoría de las pantallas del handoff, y la app
   hoy, a todo el ancho)?
1. **Toast.** ¿Abajo al centro en negro, con acción «Undo» (handoff), o abajo a la derecha con borde de
   color (app)?
1. **Reintentar.** ¿«Try again» o «Retry»?
1. **`<select>` nativo.** ¿Se permite en formularios sencillos (zona horaria, tema, «Show»), o siempre
   desplegable propio, como dice el kit?
1. **Tablas en móvil.** ¿Fila apilada como tarjeta, o *scroll* horizontal con un indicador visible?
1. **Confirmación destructiva.** ¿Botón rojo relleno (handoff) o el borde rojo actual (`btn-danger`)? ¿Y
   archivar cuenta como destructivo, que hoy unas veces es `btn-primary` y otras `btn-danger`?
1. **Enviar formularios.** ¿`btn-solid` pine para todo envío, como dice DESIGN.md, o `btn-primary`, como
   hacen casi todas las pantallas? Si es el primario, DESIGN.md debe cambiar.
1. **Nombres.** ¿«Team» o «People» para las personas? ¿«Restore» o «Reactivate» como contrario de
   archivar? ¿«Settings» o «Workspace settings»?

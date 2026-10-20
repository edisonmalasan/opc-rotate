---
name: OpenCode Recovery
description: A dark operator console for watching live OpenCode sessions and recovering rate-limited work.
colors:
  enamel-950: "#0b1210"
  enamel-900: "#101815"
  enamel-850: "#141d1a"
  enamel-800: "#19231f"
  enamel-750: "#1f2a26"
  enamel-700: "#26332e"
  enamel-600: "#2f3d37"
  ink: "#e8efea"
  ink-strong: "#f4f8f5"
  ink-muted: "#a7b5ae"
  ink-dim: "#94a39b"
  ink-faint: "#829088"
  rule: "#ffffff14"
  rule-soft: "#ffffff0a"
  rule-strong: "#ffffff2b"
  edge-hair: "#ffffff07"
  edge-soft: "#ffffff08"
  edge-scrim: "#ffffff21"
  edge-lift: "#ffffff30"
  edge-contrast: "#ffffff32"
  edge-bright: "#ffffff4d"
  running: "#b4dd97"
  running-deep: "#8fbe73"
  running-ghost: "#b4dd9726"
  running-edge: "#b4dd9733"
  signal-lime-hover: "#c3e5ac"
  on-running: "#141c11"
  held: "#e0b47e"
  held-edge: "#e0b47e3d"
  terminated: "#e38573"
  terminated-edge: "#e07a6840"
  warning-surface: "#2f2721"
  warning-border: "#6b5136"
  warning-text: "#f0cda7"
  dialog-error: "#e8b48c"
  shadow-soft: "#00000026"
  shadow-lift: "#00000033"
  shadow-modal: "#0009"
  backdrop: "#060b09c4"
  backdrop-solid: "#060b09"
typography:
  heading:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "20px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  heading-compact:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "18px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  title:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.025em"
  dialog-heading:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "21px"
    fontWeight: 600
    lineHeight: 1.5
    letterSpacing: "-0.035em"
  body:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "14px"
    fontWeight: 550
    lineHeight: 1.45
    letterSpacing: "-0.012em"
  label:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "13px"
    fontWeight: 550
    lineHeight: 1.5
  caption:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
  small:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.4
  plate:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0.04em"
  metric:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "24px"
    fontWeight: 450
    lineHeight: 1.1
    letterSpacing: "-0.03em"
  mono:
    fontFamily: '"SFMono-Regular", Consolas, ui-monospace, monospace'
    fontSize: "12px"
    fontWeight: 400
rounded:
  square: "1px"
  focus: "3px"
  tag: "4px"
  control: "5px"
  button: "6px"
  row: "7px"
  block: "8px"
  surface: "10px"
  pill: "20px"
spacing:
  xs: "4px"
  sm: "6px"
  md: "8px"
  lg: "12px"
  xl: "18px"
  2xl: "25px"
components:
  button-primary:
    backgroundColor: "{colors.running}"
    textColor: "{colors.on-running}"
    rounded: "{rounded.button}"
    padding: "8px 14px"
    height: "36px"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.signal-lime-hover}"
    textColor: "{colors.on-running}"
  button-primary-disabled:
    backgroundColor: "{colors.enamel-750}"
    textColor: "{colors.ink-dim}"
  button-secondary:
    backgroundColor: "{colors.enamel-700}"
    textColor: "{colors.ink}"
    rounded: "{rounded.button}"
    padding: "8px 14px"
    height: "36px"
    typography: "{typography.label}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.control}"
    padding: "6px 10px"
    height: "32px"
    typography: "{typography.caption}"
  text-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.tag}"
    padding: "3px 4px"
    height: "26px"
    typography: "{typography.caption}"
  icon-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink-faint}"
    rounded: "{rounded.control}"
    padding: "0"
    height: "28px"
    width: "28px"
  input-search:
    backgroundColor: "{colors.enamel-950}"
    textColor: "{colors.ink}"
    rounded: "{rounded.button}"
    padding: "0 10px"
    height: "34px"
    typography: "{typography.label}"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.button}"
    padding: "6px 10px"
    height: "36px"
    typography: "{typography.label}"
  nav-item-active:
    backgroundColor: "{colors.enamel-800}"
    textColor: "{colors.ink-strong}"
  group-heading:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    padding: "16px 26px 6px 36px"
    height: "40px"
    typography: "{typography.label}"
  row-selected:
    backgroundColor: "{colors.enamel-700}"
    textColor: "{colors.ink-strong}"
    rounded: "{rounded.row}"
    height: "56px"
    typography: "{typography.body}"
  route-line:
    backgroundColor: "transparent"
    textColor: "{colors.running}"
    rounded: "{rounded.pill}"
    height: "10px"
    width: "240px"
  route-track:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.pill}"
    padding: "0 0 16px"
    height: "150px"
    width: "296px"
  status-dot:
    backgroundColor: "{colors.held}"
    textColor: "{colors.ink-muted}"
    size: "6px"
  segment-active:
    backgroundColor: "{colors.enamel-700}"
    textColor: "{colors.ink-strong}"
    rounded: "{rounded.tag}"
    height: "28px"
    typography: "{typography.caption}"
  table-header:
    backgroundColor: "{colors.enamel-900}"
    textColor: "{colors.ink-dim}"
    padding: "11px 18px"
    height: "40px"
    typography: "{typography.small}"
  relay-row-selected:
    backgroundColor: "{colors.enamel-700}"
    textColor: "{colors.ink}"
    height: "58px"
    typography: "{typography.caption}"
  country-code:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    rounded: "{rounded.control}"
    padding: "0"
    height: "24px"
    width: "30px"
    typography: "{typography.small}"
  switch-enabled:
    backgroundColor: "{colors.running}"
    rounded: "{rounded.pill}"
    height: "20px"
    width: "34px"
  monitor-checkbox:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    size: "15px"
  tag-source:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    rounded: "{rounded.tag}"
    padding: "2px 6px"
    typography: "{typography.small}"
  event-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    padding: "16px 5px"
    height: "76px"
    typography: "{typography.body}"
  error-band:
    backgroundColor: "{colors.warning-surface}"
    textColor: "{colors.warning-text}"
    padding: "9px 24px"
    height: "40px"
    typography: "{typography.caption}"
  empty-state:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    padding: "34px 28px"
    height: "220px"
    typography: "{typography.label}"
  dialog:
    backgroundColor: "{colors.enamel-800}"
    textColor: "{colors.ink}"
    rounded: "{rounded.surface}"
    padding: "24px"
    width: "540px"
    typography: "{typography.body}"
  dialog-field:
    backgroundColor: "{colors.enamel-950}"
    textColor: "{colors.ink}"
    rounded: "{rounded.row}"
    padding: "12px"
    height: "150px"
    typography: "{typography.body}"
---

# Design System: OpenCode Recovery

## Overview

**Creative North Star: "The Enamel Line"**

OpenCode Recovery is a night console built like a piece of enamel signage: a deep green-black ground from the app's own icon, porcelain ink on top of it, hairline rules instead of boxes, and one lime line that means *live*. It is a working instrument for one operator who has to know, at a glance, which sessions are blocked and how sure the app is about each one — so the interface is ruled, tabular, and quiet, and it says nothing it cannot back up.

The world is a departure board, not a dashboard. A session in recovery is not a status word in a column; it is a **position on a line**: six stations — Queued, Idle wait, New route, Verify IP, Deliver, Receipt — with the travelled part solid, the current station a ring, and the confirmed receipt an interchange node. Read down the column and every session's place in its recovery is legible at once. When a run breaks, the line breaks with it: a vermilion square where the route stopped, and hairline where the rest of the journey never happened. That is the whole product, drawn once.

Everything else follows from that. The ground is the icon's own green-black, so the app is committed rather than generically dark. Rules are hairlines, not borders around cards. Figures — session identifiers, paths, addresses, times, counts — are set in a tabular monospace so columns line up like an instrument readout, and prose stays in the system sans. There are no icons in rounded squares, no glass, no gradients, and no glow. Lime appears only where something is happening right now, which is why six lime checkboxes could never work: the accent has to mean one thing.

**Key Characteristics:**
- One continuous green-black enamel ramp, taken from the app icon; no neutral grey anywhere in the surfaces.
- Porcelain ink in three legible steps, all clearing WCAG AA on every ground — never faded out for quietness.
- The route line is the signature: a session's recovery is a position on six stations, and a broken run breaks the line.
- Lime means live, and only live. Monitoring is a setting, so it is marked in porcelain, never lime.
- Hairline rules and tonal steps separate regions; there are no cards, no icon tiles, and no rounded containers.
- Identifiers, addresses, paths, times, and counts are tabular monospace; prose is the system sans.

## Colors

The palette is one continuous enamel ramp, one porcelain ink ramp, and three line inks. The enamel comes from the app icon's own ground (`#16201b`), so the application's colour is literally the colour of its mark rather than a generic dark theme.

### Primary
- **Enamel** (`#0b1210` → `#2f3d37`): seven steps of the app's own green-black. `enamel-950` is the rail, `enamel-900` the canvas, `enamel-850` the bands, `enamel-800` the panel, `enamel-750` a control, `enamel-700` a selected row, `enamel-600` the selected row under raised contrast. Every surface is a step on this one ramp; none is neutral grey.
- **Rule** (`#ffffff14`, with `#ffffff0a` and `#ffffff2b`): the enamel grid. One separator colour with a soft and a strong step; regions are divided by rules and tonal steps, never by boxes.

### Secondary
- **Porcelain** (`#e8efea` → `#829088`): the ink. `ink` is a session title, `ink-strong` a title on a selected row, `ink-muted` a label, `ink-dim` metadata and timestamps, `ink-faint` icons, dots, and other non-text marks only. All three text steps clear 4.5:1 against every enamel ground, so hierarchy is carried by size and weight rather than by fading text into illegibility.

### Tertiary
- **Running** (`#b4dd97`, with `#8fbe73` for the travelled part and `#c3e5ac` for hover): the live line. It marks the current stage, a verified receipt, a connected route, and the primary action — and nothing else. The hover step lifts the fill without shifting its hue.
- **Held** (`#e0b47e`): amber for anything waiting on the operator — a rate-limited session, a deferred retry, a filter count, a caution dot.
- **Terminated** (`#e38573`): vermilion for a route that broke. It is the app's only critical colour and it is new: failed, stopped, and uncertain previously shared one amber, which made "the provider blocked this" and "we gave up" indistinguishable.
- **Warning band** (`#2f2721` surface, `#6b5136` border, `#f0cda7` text): the error banner's warm ground, and the deeper warm paragraph inside the inspector.
- **Dialog error** (`#e8b48c`): a validation failure inside the prompt editor, where the error cannot live in a band.

### Named Rules

**The Lime Means Live Rule.** Signal Lime appears only where something is happening right now: a recovery in progress, a verified receipt, a connected route, the focused control, the primary action. A setting that stays set is not live, so monitoring is marked in porcelain. If a screen shows more than two lime elements at rest, something is wrong.

**The Break Rule.** A vermilion mark means the route stopped there. It is never used for a route that is merely waiting; waiting is held amber and a completed run is solid lime.

**The Three-Step Ink Rule.** Text is never faded below legibility. Three steps of ink, all above 4.5:1 on every ground; hierarchy comes from size, weight, and mono.

**The One-Ramp Rule.** A surface tone is never invented. Take the nearest step on the enamel ramp, or use the canvas. There is no "slightly different grey" in this system.

## Typography

**Display Font:** Segoe UI Variable (with SF Pro Text, -apple-system, system-ui)
**Body Font:** Segoe UI Variable (with SF Pro Text, -apple-system, system-ui)
**Label/Mono Font:** SFMono-Regular (with Consolas, ui-monospace)

**Character:** one native variable UI family carries every word, with weight and tracking doing the work — 450 reads as data, 550 as content, 650 as a title — and a tabular monospace reserved for everything the eye compares vertically: session identifiers, project paths, public addresses, latencies, counts, and timestamps. The split is the point: words are read, figures are scanned, and the two never look alike.

### Hierarchy
- **Heading** (650, 20px, 1.2, −0.035em): the toolbar's view title. One per screen, dropping to 18px when the toolbar wraps.
- **Title** (600, 20px, 1.3, −0.025em): the inspector's subject.
- **Dialog Heading** (600, 21px, −0.035em): the prompt editor's heading.
- **Body** (550, 14px, 1.45, −0.012em): session titles, primary readouts, event prose. Session titles clamp to two lines, because a long title is often the only identifying detail present.
- **Label** (550, 13px, 1.5): navigation items, buttons, field labels.
- **Caption** (400–500, 12px, 1.5): status text, activity labels, metadata, table cells — the size that carries most of the product's information.
- **Small** (500, 11px, 1.4): table headers, source tags, the event level, the status bar.
- **Plate** (400, 15px, 0.04em): the relay's country plate, the one place a two-letter code is set larger so it reads at a glance.
- **Metric** (450, 24px, 1.1, −0.03em): the relay's ping and speed figures. The largest type in the product, and it is a number.
- **Mono** (400, 12px, tabular-nums): identifiers, paths, addresses, times, counts.

### Named Rules

**The Row Owns Its State Rule.** Status, count, and recovery position live on the row. No state is available only on hover, only in a tooltip, or only in a separate panel.

**The Weight-Not-Size Rule.** Reach for 450 → 500 → 550 → 600 → 650 before you reach for a larger size. There is no display type in this product; if a value feels too quiet, it is too dim, not too small.

**The Identifier-Is-Mono Rule.** Anything that identifies or measures is monospaced. A session id, a path, an address, a timestamp, and a count are never set in the proportional sans.

## Layout

The window is a fixed three-column frame that never scrolls as a page — only its regions scroll. `100dvh` tall, `overflow: hidden`, minimum 400px of height, minimum 360px of width.

**Columns.** Rail `232px` fixed → workspace fluid → inspector `320px` fixed. The workspace carries a `25px` gutter, so the toolbar, route bar, and view controls align with the list beneath them.

**Bands.** Toolbar `66px` → route bar `66px` → view controls `56px` → the scrolling body → status bar `28px`. Every band is `flex: 0 0 auto`; only the body scrolls. The bands sit on `enamel-850` and the working area on `enamel-900`, so the deepest plane is the one the operator works in.

**The session grid.** A row is `1fr | 240px | 56px | 12px` with a `16px` gap: identity, route, updated, chevron. Rows are `56px` tall, inset `12px` from the pane with `36px` of left padding to clear the monitoring mark. The identity column is the flexible one — it takes whatever the window gives it and the title wraps rather than pushing anything else. The identity cell carries the title on the first line, and the activity state, tab, and session id on the second, the id in monospace. The sticky column header uses the *same* grid declaration and the *same* content box as the rows, so header and rows cannot drift apart.

**The route column.** `240px`, fixed, so every lane starts and ends on the same x. The lane's stations sit on the column edges, so the first station lines up with the label under it and the last with the column rule. The updated time takes a narrow `56px` column of its own and the chevron the last, so there is no dead track between the diagram and the time.

**The flat sheet.** The list is a flat run of session rows: no project headings, no cards, no group boxes. The rail already groups by project and by source, so repeating that structure in the list only produced the same project name twice with two different tags. Each row therefore names its own project and carries a source badge on the title line, and the project path lives in the inspector. The rail's sections are **Orca projects** and **Other projects**, because that is what they list.

**The route bar.** The connection facts sit together on the left - status and host, then the public address - and the actions on the right, so the slack falls at the end of the band instead of between two things that belong together.

**Relays.** A full-width table with a sticky header on the canvas colour; cells are `58px` tall. Numeric columns are right-aligned, the address column is monospace, and the address column is dropped below 1250px.

**Activity.** One scrolling column, `22px 28px 32px` padded, with events as full-width rows separated by hairlines.

**Dialog.** `min(540px, 100vw − 36px)` wide, capped at `100dvh − 40px` with internal scrolling.

**Responsive.** Width drives the layout, and the identity column flexes at every step so nothing can overflow:
- `1250px` — rail `200px`, inspector `280px`, route column `208px`.
- `1100px` — the updated-time column is dropped and the route column narrows to `190px`.
- `950px` — rail `184px`, inspector `264px`, route column `172px`, country codes hidden.
- `760px` — the rail collapses to a `62px` icon column, the inspector becomes a full-width panel below the list, and the route column narrows to `132px`.

The desktop minimum is `980 × 660`, and nothing may overflow horizontally there or at `390 × 844`.

## Elevation & Depth

Flat by default, and flatter than before: the interface has no cards, no glass, and no glow. Depth is tonal — the enamel ramp climbs from the sunken input surface through the canvas and the bands to the panel and the modal — and separation is done with hairline rules at `8%`, `8%`, and `17%` white. A shadow appears in exactly three places, and each one means the same thing: *this is a physical control or it floats above the workspace*.

### Shadow Vocabulary
- **Modal** (`0 26px 85px #0009, inset 0 1px #ffffff07`): the prompt dialog only, over a `#060b09c4` backdrop with a 4px blur.
- **Raised control** (`0 1px 2px #00000026, inset 0 1px #ffffff08`): secondary buttons.
- **Primary control** (`0 1px 3px #00000033, inset 0 1px #ffffff30`): the lime button — the inset highlight gives the fill its physical edge.
- **Focus ring** (`0 0 0 1px #b4dd97` and `0 0 0 3px #b4dd9726`): the current route station and the focused inspector station.

### Named Rules

**The Flat-By-Default Rule.** Surfaces are flat at rest. A shadow means *this floats* or *this is a control* — never *this matters more*.

**The Hairline Rule.** Regions are separated by a 1px rule, not by a shadow and not by a tonal jump alone. A tonal step without a hairline reads as noise.

## Shapes

Radii are small and disciplined: `1px` for the two marks that must read as hard edges — a broken route's terminus and a selected row's leading rule — then `3px` for a focus outline, `4px` for tags and segmented segments, `5px` for quiet buttons, icon buttons, selects, and country codes, `6px` for primary buttons, inputs, and nav items, `7px` for session rows, `8px` for the quoted error block, `10px` for the modal and the dialog symbol, `20px` for the switch track. Fully round is reserved for status dots and the switch thumb.

There are **no icon tiles**. The previous system put a rounded square behind every glyph — the app mark, the session symbol, the route symbol, the event icon, the country code — each with a different radius, which read as decoration rather than information. Now a glyph is a glyph: the session row carries no icon at all, because the source is already named by its group, and the country code is a monospaced label in a hairline box because it is a code and not an illustration.

Borders are directional where they appear at all: a dark top edge and a faint light bottom edge read as a bevel under a light source above the window.

## Components

### Buttons
- **Shape:** `6px` radius, `36px` minimum height, `8px 14px` padding. `5px` radius and `32px` for the quiet variant, `26px` for the borderless text button, `28 × 28px` for an icon button.
- **Primary:** Signal Lime fill with `#141c11` text and a `#c3e5ac` border. One per surface.
- **Hover / Focus:** hover lifts the fill to `#c3e5ac`; secondary buttons step `#26332e` → `#2f3d37`. The focus ring is `2px` Signal Lime at `3px` offset. Pressed controls scale to `0.975`, but only when they are not focused, so keyboard activation never triggers a transform.
- **Disabled:** `#1f2a26` with dim ink and no shadow. Disabled never means "faded lime".

### Inputs / Fields
- **Style:** the search field is `230px` wide, `34px` minimum height, `6px` radius, sitting on the deepest enamel with a hairline rule. The dialog's textarea is `150px` minimum height on the same ground with a stronger rule, and is the only resizable field in the product.
- **Focus:** the search field focuses as a unit — the border lifts and the inner input drops its own outline.
- **Error / Disabled:** fields do not turn warm on error. A warm band appears above the content.

### Navigation
- **Style:** a `232px` rail on the deepest enamel with a hairline on the right. Main items are `36px` tall with a `6px` radius; project entries are `30px` and carry a bare folder glyph.
- **States:** default is secondary ink with a faint icon; hover raises the ground; active is `enamel-800` with a lime icon. Section labels are `11px` weight 600 in sentence case — never uppercase, never tracked.

### Session Rows
- **Shape:** `7px` radius, `56px` minimum height, `1fr | 240px | 56px | 12px`.
- **Background:** transparent at rest, `enamel-850` on hover, `enamel-700` with a strong rule when selected, marked by a `2px` porcelain inset on the leading edge.
- **Identity:** the title in ink, with the activity state, tab, and session id on the second line, the id in monospace. The identifier is present because the product's whole claim is exact delivery to one session, and an exact claim needs the exact id visible.

### The Route Line
- **Shape:** a `240px` horizontal lane of six stations, `10px` tall, with the station markers sitting on the column edges.
- **States:** a travelled station is a solid `#8fbe73` dot on a solid segment; the current station is a `9px` lime ring on a lime segment; a future station is a hollow dot on a hairline; a held terminus is amber; a broken terminus is a vermilion square with the following segment *removed*, so the line visibly stops.
- **Skipped stations:** a recovery that never changes the public IP leaves the two relay stations as short hairline dashes *and breaks the rule across them*, so the route that was not taken is visible rather than merely absent. The station count never changes, so lanes stay aligned across rows.
- **The live station breathes:** while a recovery is moving, the current station pulses gently. `prefers-reduced-motion` still stops it.
- **Rule:** the lane's presence is the signal that a session is in recovery. A session that is only being watched shows no lane at all.

### The Route Track
- **Shape:** the same six stations, vertical, in the inspector, each with its name and a connecting rule.
- **Rule:** the current station is marked **now**. It is the only place in the product where the stations are named; the row shows only position, so the list stays quiet.

### Monitoring Mark
- **Shape:** a `15px` custom checkbox with a hairline rule, filling porcelain with a dark tick when checked, and a short bar when the choice is mixed.
- **Rule:** it is deliberately *not* lime. Monitoring is a standing choice, not a live event, and lime is reserved for what is happening now.

### Relay Table
- **Style:** full width, tabular numerals throughout, `58px` rows, and a sticky header on the canvas colour so it does not read as a raised plane.
- **Row anatomy:** a monospaced country code in a hairline box, a bold country with its host beneath, and right-aligned latency and speed columns.
- **Selection:** the same `enamel-700` tonal step as a session row, with a `1px` inset light edge on the first cell.

### Status Indicators
- **Style:** `6px` dots — `#76787d` idle, `#b5b6b9` working, `#d9ae7f` rate-limited, `#94a39b` unknown status, `#91ba9c` connected, `#e5b885` needs attention.
- **Worker Idle:** a session whose turn finished cleanly gets a *hollow* dot in porcelain rather than a filled one. It is not a failure and not a warning - it is an offer.
- **Rule:** a dot never carries meaning alone. It always sits beside the word **Idle**, **Working**, **Worker Idle**, **Rate limited**, **Provider blocked**, or **Status unavailable**.

### Watcher Choice
- **Shape:** two controls. The app-wide one is a single button in the toolbar, immediately beside the Monitoring switch, reading **Recovery** followed by the current mode. The per-session one is a three-option segmented control in the inspector: **Rate limits**, **Worker Idle**, **Default**, with a note that says whether the session is following the app-wide mode or has its own. The label, the control and the note each sit on their own line, so nothing in the row can stretch and a mode button never grows a tall block.
- **Rule:** the two modes are **Rate limits** and **Worker Idle**, and the choice is never a colour. A watcher that does not touch the network is also the one that removes the VPN route from the screen, and the interface says so by having nothing to show.

### Continuation Prompt
- **Shape:** one prompt, not one per mode. The inspector shows the prompt that will actually be sent, with a plain line beneath it: **Used by every session** beside **Edit the default**, or **This session's own prompt** beside **Use the default**.
- **Rule:** the editor on screen is never about a different prompt than the one being sent. There is no mode wording in the prompt's own interface, because the mode decides when the prompt goes out, not what it says.

### Activity Rows
- **Style:** `76px` minimum-height rows separated by hairlines, with a `24px` severity marker on the left, the level above the prose, and a monospaced timestamp on the right.
- **Severity:** info is faint, success is lime, warning is amber, and error is vermilion — one shape, four colours.

### Error Bands
- **Style:** a full-width band with `9px 24px` padding. The connection notice is neutral enamel; the error banner is the warm `#2f2721` ground with a `#6b5136` rule and `#f0cda7` text.
- **Rule:** a band reports a condition. It is not a dismissible toast and it does not float over the workspace.

### Dialog
- **Style:** a native `<dialog>` at `540px`, `10px` radius, on `enamel-800`, with a strong rule and the modal shadow over a `#060b09c4` backdrop.
- **Behavior:** Escape closes it, the heading is `21px`/600, and the actions sit in one row at the bottom.

### Empty States
- **Style:** centred, `220px` minimum height, a faint glyph, a `550` weight heading, and a `210–290px` measure of label-size copy.
- **Rule:** an empty state says what would appear here. It never apologizes, never suggests a tour, and never shows an illustration.

## Do's and Don'ts

### Do:
- **Do** keep Signal Lime scarce — it means live, and live is rare.
- **Do** put the route lane, the state, and the counts on the row itself.
- **Do** set identifiers, paths, addresses, times, and counts in tabular monospace.
- **Do** use the weight axis (450 → 500 → 550 → 600 → 650) before adding size.
- **Do** remove a column from the grid when the window narrows, rather than compressing it.
- **Do** keep waiting states static and reserve the spinner for live connection or delivery work.
- **Do** honour `prefers-reduced-motion` (the spinner stops, transforms are removed, transitions shorten to 80ms and are limited to colour) and `prefers-reduced-transparency` (the dialog backdrop loses its blur).
- **Do** keep the `25px` gutter, the `16px` row gap, and the `56px` row.
- **Do** use a hairline to separate regions, and a tonal step to say which plane is nearer.
- **Do** wrap long paths, hostnames, and error text with `overflow-wrap: anywhere`; reserve ellipsis for single-line fields.
- **Do** take a new surface tone from the enamel ramp, never from a fresh hex value.

### Don't:
- **Don't** introduce a second saturated hue. A new state is lime, held, or terminated.
- **Don't** put an icon in a rounded square. Glyphs are glyphs; the previous system used six different boxes for six icons and none of them carried information.
- **Don't** add gradients, glass, glow, or card borders. The only blur is the dialog backdrop.
- **Don't** run a continuous spinner for a wait, a cooldown, or an awaiting-check state.
- **Don't** use lime for a setting. Monitoring is not live.
- **Don't** fade text to get quiet. Three ink steps, all above 4.5:1, and hierarchy by size and weight.
- **Don't** use one warm colour for two different failures. Rate-limited is held; stopped and uncertain are terminated.
- **Don't** redraw, recolour, or scale the app icon — it is the source of the enamel ramp and of the lime.
- **Don't** present preview fixture data as live, and don't drop the preview badge.
- **Don't** add display type, decorative large numerals, or a hero treatment. This product has no marketing surface.
- **Don't** soften a stopped or uncertain state. The wording is **Stopped · 2-IP limit** and **Awaiting next check**, and the line breaks where it stopped.

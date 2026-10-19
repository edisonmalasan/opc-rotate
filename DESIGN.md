---
name: OpenCode Recovery
description: A dark operator console for watching live OpenCode sessions and recovering rate-limited work.
colors:
  signal-lime: "#b4dd97"
  signal-lime-hover: "#c3e5ac"
  on-accent: "#1e1f20"
  verified-green: "#91ba9c"
  caution-amber: "#e5b885"
  rate-limited-amber: "#d9ad80"
  warning-surface: "#392c26"
  warning-border: "#624831"
  warning-text: "#e6b893"
  text-primary: "#ececed"
  text-secondary: "#b5b6ba"
  text-tertiary: "#a4a6ac"
  hairline: "#ffffff12"
  canvas: "#202021"
  sunken: "#1c1c1d"
  sidebar: "#191a1b"
  raised: "#252627"
  panel: "#262728"
  dialog: "#2d2e2f"
  control: "#3b3c3f"
  selected: "#303234"
typography:
  heading:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "1.25rem"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  title:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "1.25rem"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.025em"
  body:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "0.875rem"
    fontWeight: 550
    lineHeight: 1.45
    letterSpacing: "-0.012em"
  label:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "0.8125rem"
    fontWeight: 550
    lineHeight: 1.5
  caption:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.5
  small:
    fontFamily: '"Segoe UI Variable", "SF Pro Text", -apple-system, BlinkMacSystemFont, system-ui, sans-serif'
    fontSize: "0.6875rem"
    fontWeight: 500
    lineHeight: 1.4
  mono:
    fontFamily: '"SFMono-Regular", Consolas, ui-monospace, monospace'
    fontSize: "0.75rem"
    fontWeight: 400
rounded:
  xs: "4px"
  sm: "5px"
  md: "6px"
  lg: "7px"
  xl: "10px"
  2xl: "14px"
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
    backgroundColor: "{colors.signal-lime}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.md}"
    padding: "8px 14px"
    height: "38px"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.signal-lime-hover}"
    textColor: "{colors.on-accent}"
  button-primary-disabled:
    backgroundColor: "#34363a"
    textColor: "#a1a3a8"
  button-secondary:
    backgroundColor: "{colors.control}"
    textColor: "#dadadb"
    rounded: "{rounded.md}"
    padding: "8px 14px"
    height: "38px"
    typography: "{typography.label}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.sm}"
    padding: "6px 9px"
    height: "34px"
    typography: "{typography.caption}"
  input-search:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.md}"
    padding: "0 11px"
    height: "36px"
    typography: "{typography.label}"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.md}"
    padding: "7px 10px"
    height: "40px"
    typography: "{typography.label}"
  nav-item-active:
    backgroundColor: "#ffffff0d"
    textColor: "#f6f6f7"
  row-selected:
    backgroundColor: "{colors.selected}"
    textColor: "#f6f6f6"
    rounded: "{rounded.lg}"
  status-dot:
    backgroundColor: "{colors.rate-limited-amber}"
    textColor: "#d8b28a"
    size: "6px"
  segment-active:
    backgroundColor: "#3d3e40"
    textColor: "#ebebec"
    rounded: "{rounded.xs}"
    height: "32px"
    typography: "{typography.caption}"
  switch-enabled:
    backgroundColor: "{colors.signal-lime}"
    rounded: "{rounded.pill}"
    height: "22px"
    width: "36px"
  tag-source:
    backgroundColor: "transparent"
    textColor: "{colors.text-tertiary}"
    rounded: "{rounded.xs}"
    padding: "2px 5px"
    typography: "{typography.small}"
---

# Design System: OpenCode Recovery

## Overview

**Creative North Star: "The Night Console"**

OpenCode Recovery is a night console. It sits in a dark room, beside work that is still running, and it is read rather than admired. The interface is a dark instrument panel whose only job is to make one invisible condition legible: a session stopped by a provider limit, and what the app is doing about it. Every value the operator needs — status, count, stage, timing — is on the row itself, at the moment they look for it, with no hover, no tooltip, and no separate screen to cross-reference.

The personality is restrained, precise, and quietly competent. Nothing is styled for effect. There is no gradient, no glow, no glass, no illustration, no marketing surface, and no celebratory success state anywhere in the product. The interface earns its calm by being boring in the right way: it uses the operating system's own native UI type, it separates regions with hairlines instead of shadows, and it lets exactly one saturated color carry all of the meaning. When the app says **Stopped · 2-IP limit**, the interface does not soften it, animate it, or dress it up.

Density is a feature, not a compromise. The target is a single expert operator on a 1280 × 820 desktop window who is already watching five live sessions, so the design optimizes for recognition speed over breathing room: a 72px session row carries a checkbox, a status glyph, a two-line title, an activity state, a recovery stage, a timestamp, and a chevron, and it stays legible at 100% zoom without horizontal scrolling. When something must be removed for a narrower window, the column is removed from the grid outright — never squeezed until it becomes unreadable.

**Key Characteristics:**
- One saturated accent (Signal Lime) with a strict job; everything else is charcoal, neutral, or a status hue.
- State is carried by the row, at caption size, in plain language — never by an icon alone, a tooltip, or a hidden panel.
- Depth comes from tonal steps and hairlines; shadows are reserved for the modal and for physical controls.
- One variable UI family, with weight (450 → 650) carrying hierarchy inside a narrow 11–20px size range.
- Motion is short, functional, and interruptible; waiting states are static, and only live connection or delivery work spins.
- The app icon is the source of the accent and is never redrawn, recolored, or scaled.

## Colors

The palette is a neutral charcoal ramp with exactly one saturated hue and two muted status hues. Nothing else enters the surface stack.

### Primary
- **Signal Lime** (`#b4dd97`): the only saturated color in the system, and it means exactly one thing — *this is live and actionable right now*. It is the primary button's fill, the focus ring, the enabled monitoring switch track, the active navigation icon, and the checkbox accent. It is never a panel background, a large fill, or a decorative edge. Its hover step (`#c3e5ac`) lifts it, never desaturates it.
- **On Accent** (`#1e1f20`): near-black text on Signal Lime. The pair must always be used together; Signal Lime is never paired with white.

### Secondary
- **Verified Green** (`#91ba9c`): confirmation that something was checked and held. It marks the connected route dot and the **prompt sent** recovery status — states backed by a real read-back. It is a quieter relative of the accent: same family, less authority.
- **Caution Amber** (`#e5b885`): the operator must look at this, and it is not resolved. It marks the attention dot, stopped / failed / uncertain recovery status, and the status bar's recovery readout. Amber is never used for information, only for inspection.

### Tertiary
- **Rate-Limited Amber** (`#d9ad80`): the provider-failure hue, reserved for a session that has actually hit a rate limit or the free-tier client gate. It tints the session's status glyph and its activity label, and it is deliberately distinguishable from Caution Amber so that "the provider blocked this" and "the app stopped trying" never read as the same event.

### Neutral
- **Text Primary** (`#ececed`): session titles, inspector titles, and any value the operator is meant to read first.
- **Text Secondary** (`#b5b6ba`): supporting copy, buttons, and labels.
- **Text Tertiary** (`#a4a6ac`): metadata, timestamps, table cells, and placeholders — present but never competing.
- **Hairline** (`#ffffff12`): the universal separator. Quieter variants (`#ffffff05`–`#ffffff0d`) separate sub-regions; under `prefers-contrast: more` it strengthens to `#ffffff32`.
- **Canvas** (`#202021`): the workspace background behind every list.
- **Sunken** (`#1c1c1d`): inputs and search fields — the one place the surface goes *below* the canvas.
- **Sidebar** (`#191a1b`): the navigation rail, rendered translucent (`#191a1be8`) with a 24px blur so it feels like a pane in front of the app rather than a painted column.
- **Raised** (`#252627`), **Panel** (`#262728`), **Dialog** (`#2d2e2f`): the ascent above the canvas — toolbar, inspector, and modal respectively.
- **Control** (`#3b3c3f`): secondary buttons and pressed surfaces.
- **Selected** (`#303234`): the selected session row and relay row. Selection is a tonal step, not an overlay.
- **Warning band** (`#392c26` surface, `#624831` border, `#e6b893` text): the error banner. The error detail block inside the inspector deepens this to `#332c25` / `#785b3e` / `#f0cda7` for a readable warm paragraph on warm ground.

### Named Rules

**The One Accent Rule.** Signal Lime marks exactly one class of thing: what is live and actionable now. One primary action per surface. If a second thing on screen wants to be lime, one of them is wrong.

**The Uncertainty Rule.** Warm color is reserved for state the operator must inspect — blocked, stopped, uncertain, failed. Nothing merely uninteresting gets a warm hue, and no informational copy is ever amber.

**The Neutral Rule.** Everything that is not the accent or a status hue is a step on the charcoal ramp. No hue enters the surface stack, and no surface is tinted.

## Typography

**Display Font:** Segoe UI Variable (with SF Pro Text, -apple-system, system-ui)
**Body Font:** Segoe UI Variable (with SF Pro Text, -apple-system, system-ui)
**Label/Mono Font:** SFMono-Regular (with Consolas, ui-monospace)

**Character:** a single native variable UI family does all the work, with optical sizing enabled and no synthetic bolding. Hierarchy is expressed through weight and tracking rather than through more typefaces or bigger sizes; the same family at 450 reads as data, at 550 as content, and at 650 as a title. Tracking tightens as size grows (−0.035em on the toolbar heading, −0.012em on session titles) and opens for small uppercase section labels (0.05em). The monospace face is reserved for values that must be compared column-wise — public IPs, ping, counts, timestamps — and always carries tabular numerals so digits do not jitter while polling.

### Hierarchy
- **Heading** (650, 1.25rem / 20px, 1.2, −0.035em): the toolbar's view title. Exactly one per screen, and it never repeats the route bar's subject.
- **Title** (600, 1.25rem / 20px, 1.3, −0.025em): the inspector panel title and the dialog heading (21px). These name the object currently in focus.
- **Body** (550, 0.875rem / 14px, 1.45, −0.012em): session titles, primary readouts, the prompt preview. Session titles clamp to two lines rather than truncating, because a long title is often the only identifying detail present.
- **Label** (550, 0.8125rem / 13px, 1.5): navigation items, buttons, and field labels — the size of anything the operator clicks.
- **Caption** (400–500, 0.75rem / 12px, 1.5): status text, activity labels, metadata, and table cells. This is the size that carries most of the product's actual information.
- **Small** (500, 0.6875rem / 11px, 1.4): table headers, source tags, the preview badge, and the status bar.
- **Mono** (400, 0.75rem / 12px, tabular-nums): addresses, latencies, counts, and times only.

### Named Rules

**The Row Owns Its State Rule.** Status, count, and recovery stage live on the row at caption size. No status is available only on hover, only in a tooltip, or only in a separate panel.

**The Weight-Not-Size Rule.** Reach for 450 → 500 → 550 → 600 → 650 before you reach for a larger size. The system has no display type; if a value feels too quiet, it is too dim, not too small.

## Layout

The window is a fixed three-column frame that never scrolls as a page — only its regions scroll. `100dvh` tall, `overflow: hidden`, minimum 400px of height, minimum 360px of width.

**Columns.** Sidebar `224px` fixed → workspace fluid → inspector `312px` fixed. The workspace carries the full horizontal gutter of `25px`, so the three bands (toolbar, route bar, view controls) align with the list beneath them.

**Vertical bands.** Toolbar `73px` → route bar `77px` minimum → view controls `65px` minimum → the scrolling body → status bar `29px`. Every band is `flex: 0 0 auto`; only the body scrolls. The toolbar and route bar sit on raised and near-raised tones respectively so the working area reads as the deepest plane.

**The session grid.** A row is `32px | 1fr | 148px | 72px | 10px` with a `12px` gap: glyph, title and activity, recovery stage, timestamp, chevron. Rows are at least `72px` tall with `13px` of inner padding and `43px` of left padding to clear the monitoring checkbox, inset `11px` from the container edge. The sticky column-label header uses the *same* grid declaration inside the *same* scroll container, so header and rows cannot drift apart.

**Density.** Group headings are `16px 25px 8px`; consecutive groups are separated by `8px`; the list's bottom padding is `25px`. The inspector's own rhythm is `24px` of section margin with `9px` below each section label.

**Responsive.** Width, not height, drives the layout:
- `1150px` — sidebar `194px`, inspector `288px`, status column `124px`, time column `60px`, the relay IP column is dropped.
- `1100px` — the time column is dropped from both the row grid and the header grid.
- `950px` — sidebar `174px`, inspector `250px`, country codes hidden, the monitor label reduced to its control.
- `760px` — the sidebar collapses to a `62px` icon rail, the inspector becomes a full-width panel below the list, and the session row becomes a two-column grid with the glyph and chevron removed.

At each step a column is *removed from the grid*, not squeezed. The desktop minimum is `980 × 660`, and nothing may overflow horizontally there or at `390 × 844`.

## Elevation & Depth

This system is flat by default. Depth is expressed tonally: a charcoal ramp that climbs from the sunken input surface (`#1c1c1d`) through the canvas (`#202021`), toolbar (`#252627`), inspector (`#262728`), and modal (`#2d2e2f`), with hairline borders doing the actual separating work at `#ffffff12` and quieter alphas below that. A shadow is never used to mean "important" — it means one of only two things: *this floats above the workspace* (the modal) or *this is a physical control* (buttons, the active segment, the switch knob).

### Shadow Vocabulary
- **Modal** (`0 26px 85px #0009, inset 0 1px #ffffff07`): the prompt dialog only. The backdrop is `#0f0f10a1` with a 4px blur.
- **Raised control** (`0 1px 2px #0002, inset 0 1px #ffffff05`): secondary buttons.
- **Primary control** (`0 1px 3px #0003, inset 0 1px #ffffff30`): the lime primary button — the inset highlight is what gives the fill its physical edge.
- **Active segment** (`0 1px 3px #0004`): the selected segment of a segmented control, plus a `#ffffff14` top edge.
- **Active nav item** (`0 1px 2px #00000010`): the selected sidebar entry.
- **Sunken inset** (`inset 0 1px 2px #00000014`): the search field, so the input reads as cut into the surface.
- **Knob** (`0 1px 2px #0004`): the monitoring switch's thumb.

### Named Rules

**The Flat-By-Default Rule.** Surfaces are flat at rest. Adding a shadow is a decision to say "this floats" or "this is a control" — never "this matters more".

**The Hairline Rule.** Regions are separated by a 1px border, not by a shadow and not by a tonal jump alone. A tonal step without a hairline reads as noise.

## Shapes

Radii are small and consistent, so the interface reads as panels and rows rather than as cards. `4px` for tags and segmented segments; `5px` for quiet buttons, icon buttons, and selects; `6px` for primary buttons, inputs, and nav items; `7px` for session rows and the segmented track; `9px` for the route symbol; `10px` for the dialog symbol; `11px` for the app mark; `14px` for the modal; `20px` for the switch track; fully round only for status dots.

The recurring silhouette is a small rounded-square glyph container — the app mark at 37px/11px radius, the session symbol at 32×35px/7px radius, the route symbol at 36px/9px radius, the dialog symbol at 39px/10px radius — holding a single thin-stroke line icon. Icons are drawn at a stroke width of `1.5`–`1.7` and never filled.

Borders are directional rather than uniform: a dark top edge (`#0f0f1080`, `#151516`) with a faint light bottom edge (`#ffffff0c`), which reads as a bevel under a light source above the window. The selected relay row carries a single inset light edge (`inset 1px 0 #b8b9bc`) to mark its left edge without adding a second border.

## Components

### Buttons
- **Shape:** gently squared, `6px` radius, `38px` minimum height, `8px 14px` padding. `5px` radius and `34px` height for the quiet variant, `28px` for the borderless text button.
- **Primary:** Signal Lime fill with `#1e1f20` text and a `#c3e5ac` border, `0 1px 3px #0003` plus an inset top highlight. One per surface.
- **Hover / Focus:** hover lifts the fill to `#c3e5ac` (secondary buttons go `#3b3c3f` → `#4d4e50`). The focus ring is `2px solid` Signal Lime at `3px` offset, and it is never suppressed in favor of the hover state. Pressed controls scale to `0.975` — but only when they are not focused, so keyboard activation never triggers a transform.
- **Disabled:** gray `#34363a` on `#a1a3a8` text, no shadow, full opacity. Disabled never means "faded lime".
- **Secondary / Quiet / Text:** Secondary is a raised charcoal control; Quiet is transparent with a `#ffffff12` hairline; Text is borderless. All three use secondary or tertiary text, never the accent.

### Inputs / Fields
- **Style:** the search field is `220px` wide, `36px` minimum height, `6px` radius, on the sunken `#1c1c1d` surface with a `#0f0f1080` hairline, a lighter bottom edge, and an inset shadow. The dialog's textarea is `150px` minimum height, `7px` radius, with a `#5c5e6180` border.
- **Focus:** the search field focuses as a unit — its border shifts to `#9fa1a480` and it gains a `0 0 0 2px #a3a4a71a` ring — rather than the inner input drawing its own outline.
- **Error / Disabled:** fields do not turn warm on error. A warm band appears above the content, not inside the field.

### Navigation
- **Style:** a `224px` translucent rail with a `24px` blur, `3px` between items, `40px` minimum-height main items with a `6px` radius, and section labels in uppercase `12px` at weight 600 with `0.05em` tracking.
- **States:** default is tertiary text with a `#9a9b9e` icon; hover is a `#ffffff06` fill; active is `#f6f6f7` text on a `#ffffff0d` fill with a `#ffffff05` border and a Signal Lime icon. Project entries are `34px` tall and carry a `6px` folder glyph at `1.5` stroke.

### Session Rows
- **Shape:** `7px` radius, `72px` minimum height, `32px | 1fr | 148px | 72px | 10px` grid.
- **Background:** transparent at rest, `#ffffff08` on hover, `#303234` with a `#ffffff24` border when selected. Selection also brightens the title to `#f6f6f6` and the metadata to `#d0d1d3` — selection changes contrast, not just color.
- **Status glyph:** a `32 × 35px` rounded square, `#ffffff03` with a `#ffffff0a` border, tinted `#d9ad80` on `#d9ad8006` with a `#d9ad801c` border when the session is rate-limited.
- **Internal padding:** `12px 13px 12px 43px`.

### Chips / Tags
- **Style:** the source tag is a `4px` radius, `#ffffff0b` border, transparent fill, tertiary text at `small` size. The preview badge is `4px` radius on `#c2c3c50c` with a `#c2c3c52b` border.
- **State:** tags are labels, never interactive. The preview badge exists so sample data is never mistaken for a live connection.

### Segmented Control
- **Style:** a `7px` radius track on `#1717187d` with a `#ffffff04` border, `3px` of padding, and `4px` radius segments at `32px` height.
- **State:** the active segment is `#3d3e40` with a `#ffffff14` top edge and `0 1px 3px #0004`; inactive segments are tertiary text with no fill. Counts inside a segment use a warm `#edc397` at `small` size.

### Monitoring Switch
- **Style:** a `36 × 22px` track at `20px` radius with a `14px` round thumb, `3px` of padding.
- **State:** off is `#47484b` with a `#dddedf` thumb; on is Signal Lime with a `#232622` thumb translated `14px`. The track color transitions over `140ms`, the thumb over `180ms` on the standard ease-out curve.

### Status Indicators
- **Style:** `6px` dots with muted fills — `#76787d` idle, `#b5b6b9` working, `#d9ae7f` rate-limited, `#a4a6ac` unknown status, `#91ba9c` connected, `#e5b885` needs attention.
- **Rule:** a dot never carries meaning alone. It always sits beside the word **Idle**, **Working**, **Rate limited**, **Provider blocked**, or **Status unavailable**.

### Recovery Status
- **Style:** inline text at caption size within the row's status column — `#d0d1d3` by default, Caution Amber for stopped / failed / uncertain, Verified Green for sent.
- **Motion:** a `13px` spinner appears *only* for active connection or delivery work (connecting, sending a prompt). A wait, a cooldown, or a pending check uses a static clock or a static label instead. The spinner rotates at `0.8s` linear.

### Empty States
- **Style:** centered, `220px` minimum height, `#595a5d` symbol, `550` weight heading at body size in tertiary text, and a `210–290px` measure of label-size copy at 1.8 line-height. The empty state teaches the operator what would appear here rather than apologizing for its absence.

## Do's and Don'ts

### Do:
- **Do** keep Signal Lime scarce — one primary action per surface, and the accent never becomes a background.
- **Do** put status, count, and stage on the row itself, at caption size, in plain language.
- **Do** use the weight axis (450 → 500 → 550 → 600 → 650) before adding size.
- **Do** remove a column from the grid when the window narrows, rather than compressing it.
- **Do** keep waiting states static and reserve the spinner for live connection or delivery work.
- **Do** use tabular numerals for every address, latency, count, and timestamp.
- **Do** honor `prefers-reduced-motion` (the spinner stops, transforms are removed, transitions shorten to 80ms and are limited to color) and `prefers-reduced-transparency` (the sidebar loses its blur and becomes solid `#191a1b`).
- **Do** keep the `25px` workspace gutter and `12px` grid gap, and keep the row at `72px` on desktop.
- **Do** use a hairline to separate regions, and a tonal step to say which plane is nearer.

### Don't:
- **Don't** introduce a second saturated hue. A new state is neutral, Verified Green, or Caution Amber — nothing else.
- **Don't** add gradients, glows, large-radius cards, or extra blur. The only blur is the sidebar and the dialog backdrop.
- **Don't** run a continuous spinner for a wait, a cooldown, or an awaiting-check state.
- **Don't** hide a session's status behind a hover, a tooltip, or a separate screen.
- **Don't** use Caution Amber for anything that isn't asking to be inspected, and never for informational copy.
- **Don't** redraw, recolor, or scale the app icon — it is the source of Signal Lime and it is a real asset.
- **Don't** present preview fixture data as live, and don't drop the preview badge.
- **Don't** add display type, decorative large numerals, or a hero treatment. This product has no marketing surface.
- **Don't** soften a stopped or uncertain state. The wording is **Stopped · 2-IP limit** and **Awaiting next check**, and the interface says so plainly.

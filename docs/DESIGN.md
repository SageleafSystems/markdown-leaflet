---
name: Markdown Leaflet
description: A native, macOS-Finder-inspired Markdown viewer with five themes — Slate, Harbor, Nightfall, Linen, and Paper
colors:
  background-dark: "#1A1B24"
  surface-dark: "#1F2029"
  surface-alt-dark: "#262736"
  border-dark: "#363748"
  text-dark: "#E9E9EE"
  text-muted-dark: "#9A9AA8"
  background-light: "#FDFDFE"
  surface-light: "#FFFFFF"
  surface-alt-light: "#ECEFF5"
  border-light: "#D7DCE6"
  text-light: "#3A3E48"
  text-muted-light: "#6B7280"
  accent-dark: "#30D158"
  accent-light: "#2F6FED"
  on-accent: "#121610"
  resize-dark: "#FF9F43"
  resize-light: "#C2410C"
  danger-dark: "#FF6B6B"
  danger-light: "#D92D20"
  warning-dark: "#EFBA55"
  warning-light: "#B54708"
  file-icon-dark: "#81B8EB"
  file-icon-light: "#2563EB"
  folder-icon-dark: "#EDBB67"
  folder-icon-light: "#B45309"
  utility-icon-dark: "#9FC4AE"
  utility-icon-light: "#0E7490"
typography:
  chromeScale: 1.12
  title:
    fontFamily: "platform default UI sans-serif"
    fontSize: "14px"
    fontWeight: 600
  body:
    fontFamily: "platform default UI sans-serif"
    fontSize: "13px"
    fontWeight: 400
  label:
    fontFamily: "platform default UI sans-serif"
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0.04em"
rounded:
  sm: "6px"
  md: "7px"
  lg: "8px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
components:
  sidebar-row-selected:
    backgroundColor: "{colors.accent-dark}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.sm}"
    padding: "7px 8px"
  icon-button-active:
    backgroundColor: "{colors.accent-dark}"
    rounded: "{rounded.sm}"
    padding: "8px"
  search-pill:
    backgroundColor: "{colors.surface-alt-dark}"
    rounded: "{rounded.pill}"
    padding: "8px 12px"
---

# Design System: Markdown Leaflet

## Overview

**Creative North Star: "A Finder Window That Only Shows You Markdown"**

This is a native desktop app (Rust + [iced](https://iced.rs), not a web view), with a unified toolbar strip and rounded sidebar cards with 1px borders. The OS owns the window chrome. All five themes share the same layout.

The palette has five coordinated color themes: **Slate** and **Nightfall** (dark, cool), **Harbor** (dark, warm), **Linen** (light, warm), and **Paper** (light, cool). Colors can be overridden by the config file for each theme; layout stays fixed.

**Key Characteristics:**
- Rounded, bordered sidebar cards — no shadows on persistent panels.
- Each theme's accent color marks selection and active toggle state.
- Icons are hand-drawn inline SVG line icons, one consistent stroke weight, never emoji.
- File, folder, and utility icons use separate color roles. Selected rows use `on-accent` so their icons remain legible.
- Appearance follows the OS by default (Slate at night, Paper by day), or the user can select Slate, Harbor, Nightfall, Linen, or Paper in View. The menu persists the choice in TOML. Reload Theme applies palette edits without restarting. The preview body font is a separate choice (System, Sans, Serif, Mono; View → Preview Font) that applies only to the rendered document, never to the app chrome; code stays monospace.

## Colors

Each theme has a neutral ramp and semantic roles. The config file can override any role independently in `[theme.slate]`, `[theme.harbor]`, `[theme.nightfall]`, `[theme.linen]`, or `[theme.paper]`.

### Primary
- **Accent:** Finder Green for Slate, cyan-teal for Harbor, Frost Blue for Nightfall, deep teal for Linen, and a clear blue for Paper. Each accent drives selected sidebar/outline rows, active toolbar buttons, and `iced`'s built-in `primary`/`success` roles.

### Neutral
- **Background** (`#1A1B24` Slate / `#FDFDFE` Paper): the window's base fill, one step darker/lighter than Surface so panels read as raised-but-flat.
- **Surface** (`#1F2029` Slate / `#FFFFFF` Paper): toolbar, sidebar, inspector, and outline panel fill.
- **Surface Alt** (`#262736` Slate / `#ECEFF5` Paper): hover state for list rows and the search pill's resting fill.
- **Border** (`#363748` Slate / `#D7DCE6` Paper): the only separator the system uses — 1px, flat, between toolbar/content and around pills/menus. No shadows stand in for this role.
- **Text** (`#E9E9EE` Slate / `#3A3E48` Paper): primary reading color, body and row labels.
- **Text Muted** (`#9A9AA8` Slate / `#6B7280` Paper): placeholder text, section labels, icon default color.
- **On Accent:** theme-specific readable text/icon color placed on top of the accent fill.
- **Resize Cue** (`#FF9F43` Slate / `#C2410C` Paper, 30% opacity): a single full-height line while hovering or dragging a resize zone.
- **File Icon** (`#81B8EB` Slate / `#2563EB` Paper): document and file glyphs.
- **Folder Icon** (`#EDBB67` Slate / `#B45309` Paper): folder glyphs.
- **Utility Icon** (`#9FC4AE` Slate / `#0E7490` Paper): secondary utility actions such as editing.

Harbor, Nightfall, and Linen each tune danger/warning/resize/icon hues to their own background rather than reusing Slate's or Paper's flat values, since a shared red or amber fails contrast against at least one of the five backgrounds. See `src/ui/theme.rs` for every theme's exact values.

### Named Rules
**The Dark-on-Accent Rule.** Selected rows use the `on-accent` foreground configured for the active theme.

**Selection Rule.** Accent fill marks selection and active state. Colored glyphs may appear without selection, but they remain small and semantic.

## Typography

**UI Font:** platform default UI sans-serif (resolved by iced/cosmic-text's system font lookup — no bundled font file, no custom family declared)

App chrome and navigation use `CHROME_FONT_SCALE` (currently `1.12`) in `src/app/mod.rs`; document text keeps its own renderer scale.

**Character:** Plain system UI type throughout. No display type — this is a task surface, not a marketing one.

### Hierarchy
- **Window title:** the OS title names the current document or folder; there is no in-app toolbar breadcrumb.
- **Chrome body** (Regular 400, 13px × chrome scale): list rows, outline entries, inspector values, and controls.
- **Label** (Regular 400, 11px × chrome scale, uppercase): sidebar section headers (`OUTLINE`, `DOCUMENT DETAILS`) and the inspector's field labels.

### Named Rules
**The One Weight-Step Rule.** Chrome uses regular weight; if something needs to stand out, change size or color, not weight.

## Layout

### Spacing scale

The base spacing unit is 8px (`layout/spacing.rs`: `UNIT`), with 4px (`HALF`) as the finest step; every new gap, margin, or inset is a multiple of 4px and named from that module or a constant built on it, never a bare pixel literal. Structural gaps are one unit: the well around a gutter card and the space between the preview's scrollbar and a docked right card are both 8px, so they always match. The preview text inset is 12px (1.5 units) and the space beside the scrollbar 16px (2 units). Controls share named sizes in `layout/spacing.rs`: icon buttons, pills, search, and gutter tabs pad 8px × 12px; sibling controls and icon-to-label gaps are 8px, tightly related controls 4px, and groups of related content 16px; gutter cards pad 12px, except the file list's card, which pads 8px because its rows already carry 8px of their own (rows sit one unit from the card edge, icons two). Menus and lists are the exception: their rows are deliberately dense, so each keeps its own named set (`spacing::menu`, `spacing::list`) and does not follow the grid. Menu rows are 1px apart with 6px × 8px padding in a panel padded 5px; file and folder rows are 4px apart with 4px × 8px padding and a 16px indent per level, and a file's icon lines up with the folder icons (the chevron's slot stays blank and sits outside the highlight, so a file's highlight stays close to its icon and text at every depth, root included); outline rows pad 7px × 8px, nested under 13px-wide guide lines, one per level. Take a menu or list size from its set, never from a literal.

The document has a left and a right gutter. Both have permanent tabs and show one at a time: the left gutter's Files and Recent (icon only, no label — there's no room beside its other header controls for text too — and highlighted the same subtle way the sidebar toggle is, a tinted fill rather than the right gutter's bolder filled segment), the right gutter's Outline and Details (icon and label). A single toolbar button reveals or hides the right gutter without changing its tab. Ctrl+Page Up and Ctrl+Page Down cycle the right tabs and reveal the gutter; Ctrl+Shift+Page Up/Down do the same for the left gutter's Files/Recent tabs. Opening a single file shows its parent folder in Files, with that file highlighted. Both gutters are resizable, the left from 300–480px and the right from 340–480px, each with 12px internal padding and 8px surrounding wells. The document fills the remaining width with 12px internal padding. Below 1120px, or when the adjusted gutter widths would leave less than 420px for the preview, only the most recently activated gutter appears when both are open. Two docked gutters, once shown, hold down to 1088px before one gives way, so resizing around 1120px does not flicker. Closing the visible gutter in a window that shows only one closes the other too, so a hidden panel never appears in its place. Typing the first character in the toolbar search reveals Files; later keystrokes and clearing the field leave the panels alone. Below 720px the Files gutter, if open, becomes a mini-nav strip above the document: one horizontally scrolling row of file chips (relative path, selected chip in the accent pill) on a `surface-alt` fill with a 1px divider beneath, following the toolbar search filter. The right gutter overlays the document there so it remains reachable without squeezing the preview, and the strip stays above it. A gutter's stored width shrinks to keep 420px for the preview and returns when the window widens. All thresholds are defined once in `src/layout/breakpoint.rs`. Dragging stays continuous and preserves at least 420px for the side-by-side preview. On release, widths within 14px of an eligible 180, 220, 260, 320, 360, 400, or 480px anchor settle there; other widths remain exact. The toolbar is pinned above all content and never scrolls. It uses one row in wide windows. Below 900px, search spans a second row while actions remain visible in the first. The search field fills the width available to it in these compact layouts. Back and Forward are separate icon buttons, following the sidebar toggle in that order. Open File and Open Folder collapse into one button: a left-click opens a file (the common case), a right-click opens a dropdown of both choices, the same pattern Theme and Font use. This combined Open button follows Back/Forward, in the nav header while the left gutter is open and in the toolbar otherwise (including the welcome screen), so it is available in every mode. A New button — a plain "+" — sits in the nav header only (no closed-toolbar copy, since creating something needs an open destination to create it into): a left-click creates a new Markdown file, a right-click opens the same File/Folder menu a footer button here used to, now positioned under this button rather than flipped upward to clear the nav's bottom edge. Recent has no toolbar button of its own at all, in either mode, open or closed — only the left gutter reaches it, and the gutter itself is one toggle press away. In a folder, that means switching the gutter's Files/Recent tab; clicking the tabs' already-active one closes the gutter — this doubling as the toggle is why the left tabs, unlike the right, close on a reclick. The welcome screen shares that same left gutter — same `layout_plan`, same resizing, same docked/overlay behavior — except it has no Files tab: before a folder is open there's nothing for Files to show, so the panel always holds Recent, and its header is lighter than Folder's (toggle, Open, and a plain "Recent" label — no tab capsule, no Back/Forward). Recent, in both modes, lists the same remembered files and folders as sidebar rows: file or folder icon, name, and the muted parent path beneath it; opening one works exactly like Open Recent always has. With nothing remembered yet, the panel reads "No recent items" in place of a blank list. The left gutter's floor is 300px, not the general 180px, so its header — toggle, Back, Forward (Folder) or just toggle and Open (Welcome), the combined Open button, New, and the Files/Recent tabs where there are any — always has room. Whenever a dropdown or right-click menu is open, every tooltip in the toolbar and gutter headers is suppressed app-wide, rather than risk one hanging over the menu it just opened. A Theme button (a half-filled circle) and a Font button (a capital T) sit just left of the copy-path button, in the toolbar while the right gutter is closed and in that gutter's header while it is open. Both are Cycle Buttons (see **Cycle Button**): a click steps to the next theme or font, and a right-click opens the same style of dropdown, one row per choice with the current one checked (fonts as "Name · face"). Recent entries in the in-window File menu use short labels so long paths do not widen the controls; the macOS system menu retains full path context. A subtle footer surface with a thin top divider shows file and heading counts on the left, and the app name and version on the right. When the open file's frontmatter block fails to parse, a small `warning`-colored triangle glyph sits beside those counts, with a tooltip naming why — a status note, not an error banner, since the document still renders.

On macOS, the application menu bar groups About and Quit under the app name, opening and recent locations under File, panels and appearance under View, and shortcuts under Help. On Linux, compact File, View, and Help titles sit above the toolbar and open 230px floating command menus with separators, checkmarks, and shortcuts. Once a menu is open, hovering another title switches to it. Hovering Open Recent opens its submenu, which sits beside File or replaces it in windows narrower than 490px. Recent names stay on one line inside the panel. Clicking outside or pressing Escape closes the menu. Reload Theme reads the configured palette from disk; appearance choices are saved in the TOML config.

### Tooltips
Toolbar icon buttons show a hover tooltip below the control: a 6px-radius chip with a 1px `border`, `surface-alt` fill, `text` color, 4px × 8px padding, an 8px gap from the button, and a soft shadow so it reads over any content. Text is 11px × chrome scale, one line. When the control has a keybinding, its chord is appended after two spaces, e.g. "Hide file list  ⌘B" — both the label and the chord come from `domain::keymap`, never a literal string at the call site.

A control whose state is worth naming — the Theme and Font Cycle Buttons — shows that value instead of an action label, e.g. "Theme: Harbor", "Font: Source Code Pro". Long values are shortened from the middle (keeping the start and end, where a name's distinguishing part usually is) rather than the end, since the end of a font or theme name is rarely what tells it apart. The tooltip stays one line, like every other toolbar tooltip, and carries no "Cycle …" action text or "right-click for list" hint — the dropdown mark on the button is the only affordance for that.

### Sidebar Well
The left gutter sits in a 8px `background`-colored gap along the window's left edge, top, and bottom — a "well" — rather than flush against them. An 8px full-height resize target is laid over the card's own right edge, on its inner padding, so it takes no layout width and never covers content. The target is invisible at rest but always uses the horizontal resize cursor and accepts a press. Hover or drag reveals one full-height 2px orange line, drawn directly over the card's own border on the edge facing the preview: its outer edge is flush with the card's outer edge, so it covers the 1px border and 1px of fill, and nothing — least of all the 8px well margin — lies between the line and the border. The target and line are placed from `layout::budget::edge_spans`, which a test pins. The cursor change and that line are the only cues. The line fades in and out (see Motion) and remains visible through an active drag even after the pointer leaves the target. The card keeps its normal border throughout. The right gutter uses the same line and rounded card in an 8px well, with its resize target over the card's left edge. Both sides use identical geometry, mirrored. Both cards carry the same accent gradient, tilted 45° from straight down by default (`[theme.gradient] angle`). By default (`style = "border"`) a card's `surface` fill stays flat and its 1px border is the gradient: `border` through the middle, leaning toward `accent` at both ends by `opacity` (0.35 by default; 0 is no tint, 1 is pure accent). A faint accent glow spreads about 5px outside it (`glow`, 0 to 1, default 0.5; 0 turns it off). It is the only place a panel carries a shadow-like halo. The alternative, `style = "fill"`, keeps the ordinary `border` and fades the fill from flat `surface` at 60% along the gradient into `surface` mixed toward `accent` by a fifth of `opacity`.

## Elevation & Depth

Flat by design — no `box-shadow`/blur anywhere in the system except floating menus (a soft 16px shadow). Every panel boundary is a 1px `border` color, not a shadow. Depth between panels comes from the Background → Surface → Surface Alt lightness steps, not from elevation.

### Shadow Vocabulary
- **Overlay** (dark: `rgba(0,0,0,0.22)`, offset `0 6px`, blur `16px`): dropdown menus only.

### Named Rules
**The Flat-Panel Rule.** Toolbar, sidebars, inspector, and search pill are bordered, not shadowed. The gutter cards' gradient border (or fill) is not a shadow, and their small accent glow is the one sanctioned halo. Any other shadow on a panel (as opposed to a transient overlay) is a bug, not a style choice.

## Motion

Motion says where a surface came from or went; it never decorates. Nothing animates continuously. Every transition is short (180ms at most), is driven by one 16ms tick that runs only while something is moving, and ends exactly on its resting layout. Timing and curves live in `gutter/motion.rs` and `gutter/mod.rs`, never as literals at a call site.

| Transition | Duration | Curve |
| --- | --- | --- |
| Resize cue (line) fade, in and out | 130ms | ease-in |
| Gutter card slides in | 180ms | ease-out |
| Gutter card slides out | 180ms | ease-in-out |
| Toolbar buttons beside a sliding card | follow the card's position (below) | none of their own |
| Search field expands over the title (focus) | 180ms | ease-out |
| Search field settles back (blur) | 180ms | ease-in-out |

### Gutter slide
- **Direction:** a card slides in from its own window edge (left from the left, right from the right) and back out to it. Closing keeps drawing the card until it is fully gone.
- **A card never changes shape.** It keeps its width, its header, and its text wrapping the whole way, including while partly off-screen. It is moved, never squeezed (`ui/slide.rs::SlideLayer`).
- **Reversal:** toggling mid-slide reverses from where the card is, never restarting or jumping.
- **One reflow:** the preview takes its new width once, at the start of an open and the end of a close, never every frame. The toolbar and the card follow the slide; the Markdown does not.
- **Instant, not animated:** the compact top strip (no card), a gutter forced in or out by a window resize or breakpoint change (a window edge being dragged never waits on a slide), a drag resize of a gutter's width, and startup.
- **Reduced motion:** `[animation] enabled = false` in the config makes every slide, and everything that follows one, an instant change. iced cannot read the OS setting, so this is explicit.

### Toolbar during a slide
The toolbar holds the same buttons the card's header does, in one place or the other (see Buttons). So that nothing is swept sideways or pops in under a moving card:
- **Buttons fade.** The toolbar's copy of a gutter's buttons fades out as its card arrives over them and back in as it leaves. The fade is a function of the card's position: fully gone once the card is 60% in, and returning over the last 60% of a departure, so it reverses with the card. iced has no widget opacity, so a toolbar-coloured cover at partial opacity does the dimming.
- **Left cluster:** toggle, Back, Forward, and the combined Open button fade as a ghost at the position the closed toolbar gives them; the card slides over it.
- **Right cluster:** Font, Copy Path, and the toggle stay in their row and collapse out of it as they fade (`ui/slide.rs::Collapse`), together with the gap before them, so the controls beside them glide instead of jumping by the cluster's width. A cluster mid-collapse is inert.
- **Title glides.** The left cluster stays in the toolbar row, hidden, collapsing with its gap (the visible copy is the fading ghost), so the title moves with the card's edge instead of jumping left by the cluster's width when the nav opens or closes.
- **Search glides.** The search field and actions reserve space for each card in proportion to its slide, so they move with the card's edge. Beside an open right gutter they stop 8px short of the card's visible border.
- **The No-Jump Rule.** On the first and last frame of any transition the layout must equal the resting layout it starts from or ends at. Anything leaving a row therefore collapses out of it, gap included, rather than vanishing; anything arriving grows into it.

### Resize cue
The line fades in on hover or drag and out on leave or release (130ms ease-in, both ways). It is the only other animation, and it never delays a press.

## Shapes

Small, consistent corner radii: 6px on buttons and selectable rows, 7px on the "Open With" control, 8px on floating menus, 10px on gutter cards (see **Sidebar Well**, above), and a full pill (999px) on the search field only. The toolbar stays square-cornered.

## Components

### Buttons (icon buttons)
- **Shape:** 6px radius, 6px padding, fixed 16px icon.
- **Default:** transparent background, with `text` or a semantic icon color.
- **Hover:** `surface-alt` fill.
- **Pressed:** `border` fill.
- **Active/toggled** (gutter shown): 16%-opacity accent fill, icon tinted accent.
- **Where they live.** A control that opens or acts on a gutter sits in that gutter's card header while the gutter is open, and in the toolbar while it is closed, at the same pixel either way. Left: sidebar toggle, Back, Forward (Folder only), the combined Open button, and the Files/Recent tabs (Folder) or a plain "Recent" label (Welcome), all packed together at that end of the header rather than pushed apart — unlike the right header, nothing sits between them. The New button is the one exception: open-panel only, with no closed-toolbar copy. Right: Theme, Font, Copy Path, and the right toggle, in a recessed capsule well; Open With always stays in the toolbar. In a compact window the strip has no header, so the toolbar keeps the left cluster. As a card slides, the toolbar's copy fades (see Motion).

### Cycle Button
A toolbar icon button for a setting with a short, fixed list of choices (the Theme and Font buttons). It makes the common case one click and keeps the full list one right-click away.
- **Click:** steps to the next choice in the list's order and wraps from the last to the first, saving it exactly as picking it from the menu bar would. The next choice comes from the setting's own `next()` (`AppearanceChoice`, `FontChoice` in `src/config/mod.rs`), which shares one order with the dropdown and the menu bar.
- **Right-click:** opens a dropdown flush beneath the button, styled like the Open With dropdown: one row per choice, the current one checked (where there is a current choice), picking one applies it.
- **Marker:** a small triangle (7px, `text` at 55% alpha) in the button's bottom-right corner says a right-click does more. It takes no events, so the click still lands on the button. Only buttons with a right-click dropdown carry it; a button whose left-click opens the dropdown instead (Open With) does not.
- **Tooltip suppression:** while any dropdown or right-click menu is open, no control anywhere in the toolbar or a gutter header shows its tooltip — a hovering tooltip tends to cover the menu that just opened beside or under it. This applies app-wide, not just to Cycle Buttons.
- **Tooltip:** the active choice (see **Tooltips**), e.g. "Theme: Harbor", not the click action — the icon already says what the button does. No chord unless the action has a keybinding. The right-click dropdown is signaled by the marker alone, not by tooltip text.
- **Building one:** wrap the button in `with_dropdown_mark` and then `AnchorArea::on_right_press` (`src/ui/anchor_area.rs`), which reports the button's bounds; add an `Open<Thing>Dropdown(Rectangle)` message, route it in `update/mod.rs`, and fill a `ContextMenu` from a `<thing>_entries` builder in `update/context_menu.rs`, as `theme_entries` and `font_entries` do.

### Sidebar Rows (file list / outline entries)
- **Shape:** 6px radius, full width.
- **Default:** transparent, `text` color.
- **Hover:** `surface-alt` fill.
- **Pressed:** `border` fill, so the highlight never disappears between hover and selection. Every interactive control keeps a fill through hover, press and release.
- **Selected:** solid accent fill, `on-accent` text (see The Dark-on-Accent Rule).
- **Folder list:** nested folders form an indented tree and start collapsed; the indent sits outside the highlight, which starts at the row's own icon. Each folder uses SVG disclosure and folder icons. Names and relative paths that do not fit the gutter end in an ellipsis instead of wrapping, and re-fit as the gutter is resized. Arrow Up/Down highlights visible folder and file rows; Arrow Right expands a folder, Arrow Left collapses it or moves to its parent, and Tab toggles a highlighted folder. Keyboard file movement waits 160ms before opening a preview. Search results show a muted relative parent path under each filename; selecting a result opens its ancestor folders. Empty folders and searches with no matches explain the state in place of a blank list.

### Search Pill
- **Style:** `surface-alt` background, 1px `border`, 999px (full pill) radius, 8px/12px padding. A magnifying-glass icon (`text-muted`) sits inline at 14px, and the nested text input is rendered transparent so the pill reads as one shape, not an input-inside-a-box.
- **Position:** 220px wide at rest in a full-width toolbar, filling its row below 900px. It moves with an opening or closing gutter (see Motion) and rests 8px short of an open right gutter's border.
- **Clear button:** a × at the far right of the pill, shown only while there is text. It is a 12px `text-muted` icon with a round `border`-colored fill on hover and press, a "Clear search" tooltip, and it empties the field and keeps focus in it.
- **Escape:** with text in a focused field, Escape clears it and leaves focus in the field; with nothing to clear, Escape leaves the field (it blurs, and the expansion settles back). So one press empties it and a second leaves it.
- **Focus:** while the field has keyboard focus (however it got it) it expands, animated, to fill the toolbar room to its left, taking it from the title (see Document Title). Blurring settles it back. Only the full-width toolbar does this; below 900px the field already fills its own row.

### Document Title
- **What:** the open document's file name (e.g. `notes.md`), or its frontmatter `title` field when it has one, 13px × chrome scale, **bold**, in `text` color, one line, at the start of the toolbar's middle slot, right after the left cluster and the opposite side from the search field. Nothing when no document is open. The window's own title bar follows the same rule.
- **Position beside the nav:** with the left gutter open, the title (and the toolbar's content generally) starts 8px past the card's visible border, not past its well margin; beside an open right gutter the content stops 8px short of that card's border. Both come from `layout::budget::toolbar_padding`.
- **Width:** its own width, at most 240px, and never more than the slot leaves beside the search field. A longer name ends in an ellipsis, and the full name shows in a tooltip while it is elided.
- **With the search:** title and search share one slot, so they never overlap or leave a jump. When the search field is focused, the title shrinks to a third of its width (eliding as it goes) and the search field grows into the room it gives up; blurring reverses both. With no title the search takes the whole slot. Only the full-width toolbar shows a title; the two-row and stacked toolbars do not yet.
- **Later:** the expanded field is meant to become a command palette (keyboard-driven menu, slash commands); the title shrinking to a third is the room that reserves for it.

### "Open With" Control
- **Style:** a pencil icon button like the other toolbar icon buttons, with an "Open with…" tooltip. It opens a dropdown on a plain click, styled like the Font dropdown, one row per editor, never a remembered previous choice — there's no single "current editor" to reflect. A dropdown opens flush beneath its button, left-aligned; when it would cross the window's right edge it right-aligns to the button's right edge instead, so it always hangs directly below its button.
- **Menu:** `surface` background, 8px radius, 1px `border`, the one shadow in the system (see Elevation & Depth). Selected-row highlight is a solid accent fill with `on-accent` text, matching sidebar rows.

### Inspector Panel
- **Shape:** 220px default card width, resizable from 180–480px, 10px radius, 1px border, 12px internal padding, 8px outer well, 16px row spacing.
- **Content:** a FILE section label over one rounded group (10px radius, `surface-alt` fill, faint border) of hairline-separated rows: File Name, Full Path, File Size, Modified. Name and size are a 13px label with a `text-muted` value at the right; Full Path and Modified put the value under the label, and Full Path carries a copy button (tooltip "Copy full path") that copies the absolute path. When the open file starts with a YAML frontmatter block, a FRONTMATTER section label (with a field-count badge, matching OUTLINE's) sits between FILE and CONTENT, over its own rounded group: each field is a row in file order, a long value dropping to the label-over-value shape Modified uses; a nested map or list becomes a row showing a field or item count instead of a value, followed by its own rows indented one guide line deeper, the way OUTLINE nests headings. No block, or one that doesn't parse, means no FRONTMATTER section at all. A CONTENT section label then heads a 3×2 grid of stat tiles (Words, Characters, Lines; Headings, Links, Images), each a bold 17px figure over an 11px muted label — these count the document body only, not a frontmatter block. The Outline/Details tabs sit in a capsule well as a segmented control, the active segment filled with the accent — the same capsule the left gutter's Files/Recent tabs use, at the opposite (inner) end of that header.
- **Outline:** an OUTLINE label with a count badge, then heading rows nested under thin guide lines, one per level below the shallowest heading present. The top level is bold, the next regular, deeper levels 12px and muted.
- **Width:** the right gutter's floor is 340px and its default 360px, so the header's tabs and buttons never clip.

### Minimap

A 72 px raised panel just left of the preview's scrollbar, casting a soft shadow (28% black, 12 px blur, 2 px left) onto the text beside it, one bar per block, drawn with the detail its height allows (3 px per text line; anything shorter than two lines is a solid bar). Headings are solid accent bars, shorter as the level deepens. Paragraphs are ragged muted lines with a shorter last line. Lists add an accent bullet dot before each line. Quotes have an accent rule down the left. Code sits on a faint panel with accent lines indented in steps. Tables are a faint panel of three cells per row with a brighter header row. Images are a tinted panel with a sun and a hill. Rules are a one-pixel border line. Line lengths come from `domain::minimap::line_length`, so the picture is stable between frames. While find is open, each block holding a match gets a warning-colour tick in the strip's right margin, wider and brighter for the active match. The heading the reader is in is drawn in the text colour instead of the accent. A translucent box with a muted outline marks the viewport. Pressing or dragging centres the viewport at the pointer. It is off by default. Its strip is reserved while on and drawn only while the document scrolls, and toggles from the toolbar (lit when on), the View menu, and `Ctrl/⌘ Shift M`.

### Error Banner
- **Style:** `danger` text and border at full opacity, `danger` background at 12% opacity, 6px radius, 8px padding. The only place in the system where a semantic color (not the accent) gets a background fill.

## Do's and Don'ts

### Do:
- **Do** keep both appearances structurally identical — same spacing, same radii, same component states — differing only in the neutral ramp and accent lightness.
- **Do** use the active theme's `on-accent` color for text/icons on selected rows.
- **Do** draw new icons as single-stroke inline SVG at the existing 1.75px stroke weight, tinted via the icon's own color param rather than baked into the SVG.
- **Do** reserve the pill (999px) radius for the search field; every other interactive shape uses the 6–8px step.
- **Do** make any new transition end exactly on its resting layout, derive it from a position rather than a timer of its own where it follows another motion, and honor `[animation] enabled`.

### Don't:
- **Don't** use semantic icon colors as large panel fills; they are small navigation cues.
- **Don't** add shadows to panels (toolbar, sidebars, inspector). Boundaries are borders; floating menus are the sanctioned exception.
- **Don't** use emoji or Unicode glyphs as icons — every glyph in the toolbar, sidebar, and lists is a drawn SVG from `src/ui/icons.rs`.
- **Don't** let a transition squeeze, rewrap, or reflow content as it moves, or let a control that leaves a row vanish at once so its neighbors jump (the No-Jump Rule); move it, or collapse it out of the row.
- **Don't** put the resize line anywhere but over the card's own border.
- **Don't** draw custom traffic lights or a unified title bar. The app styles everything *below* the OS-drawn window chrome, not the chrome itself.

## Rendered Markdown

Headings, list bullets, and ordered-list numbers use the theme `accent`; body text, links, and code keep their own colors. Line height follows the golden ratio and tracks the width of the text column: body leading is `1 + fill/φ`, where `fill` is the column width over φ⁷ em (about 60–70 characters), clamped between 1.25 and φ (1.618). Headings and code lines use that leading scaled toward 1 by φ. Back and Forward chevrons follow the sidebar toggle in the toolbar and dim, with no press action, when there is nowhere to go. Links to a Markdown file inside the open folder replace the current document; other links open in the system handler.

The preview pane has no padding at all. Its scrollbar sits flush on the pane's edge, whether that edge is the window edge or a docked right gutter's card, rather than floating inside the pane, and the scrolled text runs under the toolbar and down to the window's bottom, so it is cut off at those edges with no gap. Only the scrolled content is inset, by 12 px on the left (`PREVIEW_INSET`) and 16 px on the right beside the scrollbar. The find bar uses the same left inset.

Images are drawn at their natural size and shrink, keeping their aspect ratio, to the width of the text column; they never scale up. Raster images get the same corner radius as the spacing unit's half; SVGs are drawn as they are. While an image decodes, a muted "Loading image…" box in the code-block surface holds its place. An image that cannot be shown (missing, outside the open folder, remote, unsupported, too large, or corrupt) becomes that same surface with its alt text and a muted one-line reason; a working image shows no caption. The alt text stays selectable and searchable either way.

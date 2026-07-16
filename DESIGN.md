# Koofr-GUI Design System

## 1. Atmosphere & Identity

Koofr-GUI should feel like a calm Windows file manager: compact, dependable, and immediately legible during long file operations. Its signature is Koofr green used sparingly as an operational signal against cool neutral surfaces; downloads and destructive actions favor clarity over decoration.

## 2. Color

### Palette

| Role | Token | Light | Usage |
| --- | --- | --- | --- |
| Surface/primary | `--surface` | `#ffffff` | Main application and modal surface |
| Surface/canvas | `--canvas` | `#e8eaed` | Window background |
| Surface/subtle | `--surface-subtle` | `#f5f6f7` | Sidebar and quiet controls |
| Surface/raised | `--surface-raised` | `#fbfcfc` | Settings sections and grouped controls |
| Text/primary | `--text` | `#17191d` | Primary copy and headings |
| Text/secondary | `--muted` | `#666c76` | Supporting copy |
| Text/tertiary | `--subtle` | `#8b919a` | Metadata, placeholders, disabled content |
| Border/default | `--border` | `#e1e4e8` | Dividers and control outlines |
| Border/strong | `--border-strong` | `#cfd4d9` | Form fields and emphasized boundaries |
| Accent/primary | `--accent` | `#66c451` | Primary actions, progress, selected state |
| Accent/hover | `--accent-dark` | `#50a93f` | Hover and active accent |
| Accent/contrast | `--accent-contrast` | `#142a17` | Text/icons on bright accent surfaces |
| Accent/subtle | `--accent-soft` | `#f0faed` | Selected and informational backgrounds |
| Accent/subtle-strong | `--accent-soft-strong` | `#e3f5de` | Stronger accent tint |
| Status/error | `--status-error` | `#b33d37` | Error messages and destructive feedback |
| Status/error-soft | `--status-error-soft` | `#fff1ef` | Error background |
| Status/warning | `--status-warning` | `#8a6330` | Cautionary notes |
| Overlay | `--overlay` | `rgba(20, 23, 28, 0.28)` | Modal backdrop |

### Theme accents

The `koofr`, `ocean`, `iris`, `coral`, and `berry` themes may override only the five accent tokens. Neutral surfaces, statuses, and text remain stable so file-management semantics do not shift with appearance preferences.

### Rules

- Accent colors communicate selection, progress, or an available action; they are not decorative gradients.
- Errors use the status tokens and never depend on accent theme.
- New colors must be added here before use.

## 3. Typography

### Scale

| Level | Size | Weight | Line height | Usage |
| --- | --- | --- | --- | --- |
| Display | `2.25rem` | 700 | 1.12 | Authentication heading |
| H1 | `1.625rem` | 700 | 1.2 | Workspace title |
| H2 | `1.25rem` | 650 | 1.3 | Modal title |
| H3 | `0.875rem` | 650 | 1.4 | Settings section heading |
| Body | `0.8125rem` | 400 | 1.6 | Default desktop copy |
| Body/small | `0.75rem` | 400 | 1.5 | Controls and file rows |
| Caption | `0.6875rem` | 500 | 1.45 | Metadata and helper text |

### Font stack

- Primary: `"Segoe UI Variable", "Segoe UI", "Microsoft YaHei UI", system-ui, sans-serif`
- Mono: `Consolas, "Cascadia Mono", monospace`

### Rules

- Use the Windows-native stack for a desktop-client feel and reliable Chinese glyph metrics.
- Apply tabular figures to byte counts, percentages, and transfer rates.
- Long Chinese helper copy uses natural wrapping and at least 1.5 line height.

## 4. Spacing & Layout

### Base unit

All spacing derives from a 4px base.

| Token | Value | Usage |
| --- | --- | --- |
| `--space-1` | `4px` | Icon micro-gap |
| `--space-2` | `8px` | Compact inline groups |
| `--space-3` | `12px` | Field padding and row gaps |
| `--space-4` | `16px` | Standard group spacing |
| `--space-5` | `20px` | Modal and panel breathing room |
| `--space-6` | `24px` | Large component padding |
| `--space-8` | `32px` | Major group separation |

### Grid

- Desktop shell target: 1280×720 with a 980×640 minimum native window.
- Transfer panel: 358px desktop rail, overlaying content at narrower desktop widths.
- Modal widths: 430px standard and 660px wide, constrained to the viewport.
- Responsive checkpoints: 1280px, 920px, and 650px; forms and setting choices collapse to one column at the compact breakpoint.

### Rules

- Dense file lists remain compact; settings and confirmation flows receive larger spacing.
- Input/action groups remain one row while usable, then stack without horizontal scrolling.
- Values outside the spacing tokens are reserved for icon geometry and existing Windows optical alignment.

## 5. Components

### Modal

- **Structure**: backdrop → semantic dialog section → close button, heading, content, primary action.
- **Variants**: standard, wide, and download destination.
- **Spacing**: `--space-3` through `--space-6`.
- **States**: opening, visible, busy/disabled action, validation error, closing.
- **Accessibility**: labelled modal dialog, Escape dismissal, focus entry and restoration, backdrop click dismissal, clicks inside never dismiss.
- **Motion**: backdrop opacity and dialog transform/opacity using standard timing.

### Path field

- **Structure**: visible label → text input + adjacent folder-picker icon button → helper or inline error.
- **Variants**: settings and per-download.
- **States**: default, hover, focus-within, busy, invalid.
- **Accessibility**: persistent label, described helper/error, explicit folder-picker button name.

### Choice switch

- **Structure**: copy block plus native button with `role="switch"` and visible track/thumb.
- **States**: off, on, hover, focus, disabled.
- **Accessibility**: `aria-checked` reflects the persisted setting.

### Transfer panel

- **Structure**: header, status tabs, transfer list, per-item controls, footer.
- **States**: open, hidden, empty, running, completed, failed, cancelled.
- **Accessibility**: live transfer values remain textual; action buttons have explicit labels.

## 6. Motion & Interaction

| Type | Duration | Easing | Usage |
| --- | --- | --- | --- |
| Micro | `120–160ms` | ease-out | Hover, press, switch thumb |
| Standard | `200–260ms` | ease-in-out | Modal and panel open/close |
| Emphasis | `400ms` | cubic-bezier(0.16, 1, 0.3, 1) | Rare onboarding emphasis |

- Animate only transform and opacity for new motion.
- Every clickable control has hover, active, focus-visible, and disabled treatment.
- All transient overlays close on outside click and Escape unless an operation is in an unsafe commit phase.
- `prefers-reduced-motion` collapses non-essential durations.

## 7. Depth & Surface

Strategy: mixed, restrained to native desktop hierarchy.

| Level | Treatment | Usage |
| --- | --- | --- |
| Inline | 1px neutral border, no shadow | Inputs, settings choices, rows |
| Raised | Tonal shift with 1px border | Settings sections and contextual groups |
| Overlay | `0 24px 70px rgba(20, 24, 30, 0.24)` | Modal only |
| Side rail | `-12px 0 34px rgba(28, 32, 38, 0.14)` | Transfer panel overlay |

- Shadows indicate overlays or floating controls, never ordinary content cards.
- Nested controls use tighter radii than the containing modal or section.
- Avoid decorative glass, gradients, or texture in operational file-management surfaces.

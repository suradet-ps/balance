# Balance Design System

## 1. Product Context

**Balance** is a Tauri 2 desktop dashboard (Rust/Leptos frontend built by Trunk) that unifies **HOSxP** and **INVS** into a side-by-side drug inventory comparison view for Thai hospitals. The UI is a white-background, two-panel dashboard: HOSxP data (drug quantities, purple family) on the left, INVS data (drug values, green family) on the right.

**The single source of truth for all visual tokens is `src/assets/styles/theme.css`.** Component stylesheets must reference tokens via `var(--token)` only — raw hex/rgba values anywhere else fail CI (`design-enforcement` job).

## 2. Color Palette & Roles

### Primary — Balance Purple (HOSxP side)
| Token | Value | Role |
|---|---|---|
| `--primary` | `#7132f5` | Primary CTA, brand accent, links, active tab |
| `--primary-dark` | `#5741d8` | Button hover, outlined variants |
| `--primary-deep` | `#5b1ecf` | Deepest purple (unused elsewhere — reserved) |
| `--primary-subtle` | `rgba(133, 91, 251, 0.16)` | Subtle icon/CTA backgrounds |

### Neutral
| Token | Value | Role |
|---|---|---|
| `--near-black` | `#101114` | Primary text; dark header background |
| `--cool-gray` | `#686b82` | Secondary text |
| `--silver-blue` | `#9497a9` | Muted text |
| `--border-gray` | `#dedee5` | Input borders, dividers |
| `--bg-base` | `#ffffff` | Primary surface |
| `--bg-surface` | `#f8f9fc` | KPI bar background |
| `--bg-elevated` | `#f1f2f6` | Hover fills, tab bar track, chips |

### Semantic
| Token | Value | Role |
|---|---|---|
| `--green` | `#149e61` | Success / INVS identity |
| `--green-dark` | `#026b3f` | Badge text on green |
| `--red` | `#e03e3e` | Errors, disconnect states |
| `--green-light` / `--red-light` | `#4ade80` / `#f87171` | Status dots & badges on dark header |

### Chart — semantic per side
| Token | Value | Role |
|---|---|---|
| `--chart-hosxp` | `#7132f5` | HOSxP series color, tooltip border |
| `--chart-hosxp-line` | `#5741d8` | HOSxP line series |
| `--chart-hosxp-tooltip-bg` | `#1a1040` | HOSxP tooltip fill |
| `--chart-invs` | `#149e61` | INVS series color, tooltip border |
| `--chart-invs-line` | `#026b3f` | INVS line series |
| `--chart-invs-tooltip-bg` | `#1a2e1a` | INVS tooltip fill |

## 3. Typography

### Font Families (loaded in `src/index.html`)
- **Display / UI**: `--font-display` / `--font-body` → IBM Plex Sans, fallback `Helvetica Neue, Helvetica, Arial`
- **Numeric**: `--font-mono` → IBM Plex Mono (numbers/codes)

Single font family for both headings and body — hierarchy comes from size/weight, not a separate display face.

### Hierarchy (actual usage)
| Role | Size | Weight | Notes |
|---|---|---|---|
| Brand title (header) | 16px | 700 | `letter-spacing: -0.5px` |
| KPI value | 16px | 700 | paired with 11px unit |
| Drawer title | 15px | 600 | |
| Body / buttons | 14px | 400–500 | base body `line-height: 1.5` |
| Banners | 13px | 400–600 | link buttons 600 + underline |
| Panel label / form label | 11–12px | 600 | `text-transform: uppercase`, `letter-spacing: 0.04–0.05em` |
| Caption / chips | 11–12px | 500 | |

## 4. Component Stylings

### Buttons (`.btn`)
- Base: `padding: 8px 16px`, radius `--radius-lg` (12px), 14px / 500, active scale 0.97
- **`.btn-primary`**: `--primary` bg, white text; hover → `--primary-dark`
- **`.btn-ghost`**: transparent, `--text-secondary`, `1px solid var(--border-subtle)`; hover → `--bg-elevated`
- **`.btn-secondary`**: `--bg-elevated` bg, `1px solid var(--border-gray)`; disabled → 60% opacity
- **`.btn-icon`**: icon-only; muted → primary on hover, `--radius-sm`
- **`.link-btn`**: text button, primary, 13px 600, underline

### Inputs (`.input`)
- `padding: 8px 12px`, radius `--radius-lg`, `1px solid var(--border-gray)`
- Focus: `--primary` border + `0 0 0 3px var(--primary-subtle)` ring; placeholder `--text-muted`

### Cards
- `.card`: `--bg-base`, radius `--radius-xl` (16px), `1px solid var(--border-subtle)`, `--shadow-card`; hover `--shadow-hover`
- `.chart-card`: fills panel, overflow hidden
- `.kpi-card`: radius `--radius-lg`, `--border-subtle`, padding `10px 14px`

### Badges
- `.badge`: pill (`border-radius: 999px`), `padding: 4px 10px`, 12px / 500
- `.badge-connected`: `--green-subtle` bg, `--green-dark` text
- `.badge-disconnected`: `--red-subtle` bg, `--red` text
- On dark header: translucent bgs (`rgba(20,158,97,0.15)` / `rgba(224,62,62,0.12)`) with `--green-light` / `--red-light` text

### Tabs (connection drawer)
- `.tab-bar`: `--bg-elevated` track, radius `--radius-lg`, 3px padding
- `.tab-btn`: 13px / 500, `--text-secondary`; `.active`: white bg, `--primary` text, micro shadow, 600

### Header (`.app-header`)
- 52px tall, `--near-black` bg, white text, `box-shadow: 0 2px 8px rgba(0,0,0,0.15)`
- Contains: brand (logo + title + sub), year selector (dark select), connection badges, settings button

### Charts
- Canvas-rendered (Rust, replacing ECharts) with HTML tooltip overlay (`.chart-tooltip`): tooltip-bg + series color border, `--radius-sm`, 12px text

## 5. Layout Principles

- **App shell**: column flex — header (52px) / main grid / KPI bar; `overflow: hidden` (desktop app, no page scroll)
- **Main grid**: `grid-template-columns: 1fr 1px 1fr` with 1px `--border-subtle` divider between panels; padding `12px 16px 0`
- **Drawer**: 400px right side, overlay `rgba(16, 17, 20, 0.5)`, slide-in from right (250ms)
- **Spacing (in use)**: 2, 3, 4, 5, 6, 8, 10, 12, 14, 16, 20, 24, 32, 36px
- **Radius tokens**: `--radius-sm` 6px (chips, small), `--radius-md` 10px, `--radius-lg` 12px (buttons, inputs, tabs, kpi), `--radius-xl` 16px (cards); pills (999px) reserved for badges/status dots

## 6. Depth & Elevation
| Token | Value | Used for |
|---|---|---|
| `--shadow-card` | `0 2px 12px rgba(0,0,0,0.03)` | cards |
| `--shadow-hover` | `0 4px 24px rgba(0,0,0,0.06)` | hover lift |
| — | `0 1px 4px rgba(0,0,0,0.08)` | active tab, micro |
| — | `0 8px 24px rgba(0,0,0,0.12)` | dropdown, tooltip |
| — | `0 2px 8px rgba(0,0,0,0.15)` / `-6px 0 32px rgba(0,0,0,0.15)` | header / drawer |

Transitions: `--transition-fast` (150ms) for color/bg states, `--transition-med` (250ms) for layout/shadow/overlays.

## 7. Do's and Don'ts

### Do
- Route every color through `theme.css` tokens — raw hex outside `theme.css` fails CI
- Use `--primary` for CTAs, links, active states; `--green`/`--red` strictly for positive/negative semantics
- Keep HOSxP = purple family and INVS = green family everywhere (dots, KPI icons, charts)
- Use 12px radius (`--radius-lg`) for interactive controls; pill only for badges/status dots
- Use `--font-mono` for numeric values, drug codes, and identifiers

### Don't
- Don't introduce new purples/greens outside the defined token scale
- Don't use `border-radius: 999px` on buttons or inputs (12px max)
- Don't hard-code shadows or spacings that already have tokens

## 8. Responsive Behavior

Desktop-first Tauri app; layout assumes a wide window (two-panel grid). Constraints:
- Panels keep a minimum usable width — narrow the window and panels shrink with `min-width: 0`, text truncates with ellipsis
- The connection drawer is a fixed 400px overlay regardless of window size
- `.kpi-card` values use `white-space: nowrap` and `overflow: hidden` rather than wrapping

## 9. Agent Prompt Guide

### Quick Token Reference
- Brand/primary: `var(--primary)` = `#7132f5` (hover `--primary-dark`)
- Text: `--text-primary` (`#101114`), secondary `--text-secondary` (`#686b82`), muted `--text-muted` (`#9497a9`)
- Success `--green` / `#149e61`, danger `--red` / `#e03e3e`
- Surfaces: `--bg-base` white, `--bg-surface` `#f8f9fc`, `--bg-elevated` `#f1f2f6`
- Fonts: IBM Plex Sans (UI), IBM Plex Mono (numbers)

### Example Component Prompts
- "Create a card: white `--bg-base`, `--radius-xl`, `1px solid var(--border-subtle)`, `--shadow-card`. Heading 16px 700 with `letter-spacing: -0.5px`."
- "Create a CTA button: `--primary` background, white text, `--radius-lg` (12px), `padding: 8px 16px`, hover `--primary-dark`, active scale 0.97."
- "Create an input: `1px solid var(--border-gray)`, `--radius-lg`, focus ring `0 0 0 3px var(--primary-subtle)`."
- "Create a HOSxP status badge on the dark header: translucent green bg, `--green-light` text, pill radius, 11px."

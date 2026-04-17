---
name: evlog-style
description: Minimal, developer-centric dark UI inspired by evlog.dev. Focus on structured logs, high-density data, and terminal-like aesthetics.
license: MIT
metadata:
  author: custom
---

# Evlog-Inspired Design System (Universal)

## Context and goals
Create a high-density, developer-focused interface for observability and event inspection, prioritizing readability, scanability, and speed over visual decoration.

---

## Design tokens and foundations

### Color tokens (dark-first)
Use semantic tokens only.

- `color.background` = #09090B
- `color.surface` = #111113
- `color.surface-elevated` = #18181B
- `color.border` = #27272A

- `color.text.primary` = #FAFAFA
- `color.text.secondary` = #A1A1AA
- `color.text.muted` = #71717A

- `color.primary` = #2853FF
- `color.success` = #22C55E
- `color.warning` = #F59E0B
- `color.danger` = #EF4444

- `color.event.info` = #60A5FA
- `color.event.debug` = #A78BFA

### Typography
- Font family:
  - `font.body` = Geist Sans
  - `font.mono` = Geist Mono / JetBrains Mono

- Scale:
  - 12 (dense data)
  - 13 (tables/logs)
  - 14 (default UI)
  - 16 (section headers)
  - 20 (page headers)

- Rules:
  - Logs MUST use `font.mono`
  - UI labels SHOULD use `font.body`
  - Line-height:
    - mono: 1.4
    - body: 1.5

### Spacing
- Scale: 4 / 8 / 12 / 16 / 20 / 24
- Rules:
  - Dense views MUST use 8px rhythm
  - Containers SHOULD use 16–24px padding
  - Table rows MUST NOT exceed 12px vertical padding

### Borders & radius
- Radius:
  - all: 0px (strictly straight edges)

- Borders:
  - 1px solid `color.border`
  - Prefer subtle glows over heavy shadows

### Effects
- `.glass`: Deep backdrop blur (12px+) with semi-transparent surface (75-85%)
- `.glow`: Radial gradients (20% opacity) using `color.primary` in periphery areas

---

## Component-level rules

### 1. Event Row (core component)

#### Anatomy
- Timestamp (mono, muted)
- Event name (primary text)
- Metadata preview (secondary text)
- Status indicator (color-coded dot)

#### States
- default
- hover → `surface-elevated`
- active/selected → left border 2px `color.primary`
- error → left border `color.danger`
- success → left border `color.success`

#### Behavior
- Click → expands details panel
- Keyboard:
  - ↑ ↓ navigation
  - Enter → expand
- Long text:
  - MUST truncate with ellipsis
  - Full content on hover (tooltip)

---

### 2. Log Detail Panel

#### Anatomy
- Header (event name + timestamp)
- JSON viewer
- Tabs: raw / parsed / metadata

#### Rules
- JSON MUST be syntax-highlighted
- Indentation: 2 spaces
- Scroll MUST be independent (no page scroll hijack)

#### States
- loading → skeleton lines
- error → inline alert

---

### 3. Tables (high-density)

#### Rules
- Row height: 32–40px
- Font: 13px mono or body depending on content
- Column alignment:
  - numbers → right
  - text → left

#### States
- hover → subtle background change
- selected → persistent highlight

#### Edge cases
- Overflow:
  - horizontal scroll MUST be allowed
- Empty:
  - show structured empty state (not blank)

---

### 4. Filters / Search

#### Anatomy
- Search input
- Filter chips
- Time range selector

#### Behavior
- Input:
  - debounce: 300ms
- Filters:
  - toggleable chips
- Keyboard:
  - `/` focuses search
  - `Esc` clears

---

### 5. Badges (event types)

#### Variants
- info → blue
- success → green
- warning → orange
- error → red
- debug → purple

#### Rules
- Font: 12px mono
- Padding: 4px 8px
- Radius: 999px

---

### 6. Panels / Layout

#### Structure
- Sidebar (filters/navigation)
- Main (event stream)
- Right panel (details)

#### Rules
- Layout MUST support resizing panels
- Sidebar width: 240–300px
- Right panel SHOULD be collapsible

---

## Accessibility requirements and testable acceptance criteria

- Contrast:
  - Text MUST meet WCAG AA (≥ 4.5:1)
- Focus:
  - All interactive elements MUST have visible focus ring:
    - outline: 2px `color.primary`
- Keyboard:
  - Full navigation MUST be possible without mouse
- Screen readers:
  - Event rows MUST expose:
    - role="button"
    - aria-expanded (for details)
- Tooltips:
  - MUST be accessible via focus, not only hover

---

## Content and tone standards

### Tone
- concise
- technical
- no fluff

### Labels
- MUST be explicit:
  - "Filter by status" (not "Filter")
  - "Search events" (not "Search")

### Examples
- Good:
  - "No events found for this time range"
- Bad:
  - "Nothing here"

---

## Anti-patterns and prohibited implementations

- ❌ Using light backgrounds in core log views
- ❌ Mixing serif fonts
- ❌ Large paddings that reduce data density
- ❌ Hiding critical info behind hover-only interactions
- ❌ Non-truncated long strings breaking layout
- ❌ Using color alone to convey meaning (must include text/icon)

---

## Migration notes

- Replace existing card-heavy layouts with flat surfaces
- Convert verbose logs into structured rows
- Introduce mono font ONLY in data-heavy contexts
- Normalize spacing to 8px grid
- Replace modals with side panels for inspection flows

---

## QA checklist

- [ ] All colors use semantic tokens
- [ ] Text contrast passes WCAG AA
- [ ] Event rows support keyboard navigation
- [ ] Logs use monospace font consistently
- [ ] Long text truncates correctly with tooltip fallback
- [ ] Panels are resizable and responsive
- [ ] No layout shift when loading data
- [ ] Empty states are present and informative
- [ ] Hover is NOT required to access critical info
- [ ] Focus states are visible and consistent
# ADR 0021: Desktop UI preferences and visible window edges

- Status: Accepted
- Date: 2026-08-12

## Context

M.O.E. uses a transparent, undecorated Tauri window. The visible workbench was
inset by a large responsive padding, so the OS window boundary, screenshot
capture boundary, and visible yellow shell did not match. The app also had no
device-persisted typography or language preferences, and its custom window had
no explicit resize affordance.

## Decision

1. Store non-sensitive UI preferences in a versioned local browser record.
   The supported values are validated on read and fall back independently.
2. Support Japanese and English UI chrome. Room names, participant names,
   message bodies, and AI replies remain source content and are not translated.
3. Support UI font scale from 80% through 150%, and a bounded font-family enum
   with system fallbacks. Apply both at the document root so rem-based controls
   scale consistently.
4. Keep the transparent window, but reduce its visual gutter to 12px and let the
   workbench fill the remaining viewport. Native resize drag handles cover all
   four edges and four corners. The left handle therefore begins at the visible
   Chat tree's outer edge.
5. Room, appearance, and environment popovers are mutually exclusive. Escape
   or a pointer press outside the open popover and its trigger dismisses it.
6. Artwork positioning remains logarithmic around 100%, but is bounded to
   5%-200%. Values above 200% from older in-memory state are clamped.

## Consequences

- Preferences survive restarts without entering Room data or provider state.
- The main window remains visually custom while behaving like a normal
  resizable desktop window.
- English localization covers application UI and generated status copy, but
  intentionally does not rewrite historical/user/provider content.
- A future settings backend may migrate the versioned local record without
  changing component contracts.

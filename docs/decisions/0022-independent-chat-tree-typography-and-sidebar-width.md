# ADR 0022: Independent chat/tree typography and sidebar width

- Status: Accepted
- Date: 2026-08-12

## Context

ADR 0021 introduced one root font scale and a small fixed list of font
families. That proved the preference flow, but scaling the whole application
also enlarged the room tree and settings. A fixed family list cannot represent
the scripts and typefaces installed by people in different regions. The room
tree also needs an internal width control independent of the native window
edge.

## Decision

1. Keep native window resizing and internal layout resizing separate. A
   keyboard-accessible separator in the workbench gap changes the room tree
   width from 180px through 420px. The stored width is ignored by the stacked
   mobile layout below 760px.
2. Replace the root font scale with two persisted values:
   - chat scale: 80%-150%, controlled from Preferences;
   - room-tree scale: 80%-130%, controlled by compact A- / A+ buttons in the
     tree header.
3. Keep the selected font family application-wide so one installed family can
   cover both regions. Always append the system stack as a glyph fallback.
4. Enumerate installed font families in the desktop backend with `fontdb`.
   It scans the standard system font locations on Windows, macOS, and Linux.
   Return only bounded, non-control family names, remove duplicates, and sort
   them before crossing the Tauri command boundary.
5. Migrate the version-1 local preference record into a version-2 record. A
   previous root font scale becomes the chat scale; the room tree starts at
   100% and 220px.

## Consequences

- Enlarging conversation text no longer changes the room tree.
- The room tree can be widened without changing the OS window size.
- Font availability follows the current device and locale instead of a
  product-maintained allowlist.
- A selected font without a glyph for some script safely falls through to the
  operating system font stack.

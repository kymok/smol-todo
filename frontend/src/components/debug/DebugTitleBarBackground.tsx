import { TITLE_BAR_HEIGHT } from "../../layout";

// Debug-only visualization of the title-bar / window-drag strip. Dragging
// itself is handled by startWindowDrag on the root, so this sits behind
// everything (zIndex: -1) and ignores pointer events.
//
// Visibility:
//   - Production builds: never shown. `import.meta.env.DEV` is statically false,
//     so Vite dead-code-eliminates the whole overlay.
//   - Local dev: opt in by setting VITE_DEBUG_TITLEBAR=1 (e.g. in
//     frontend/.env.local). Off by default.
export function DebugTitleBarBackground() {
  if (!import.meta.env.DEV || import.meta.env.VITE_DEBUG_TITLEBAR !== "1") {
    return null;
  }
  return (
    <div
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        height: TITLE_BAR_HEIGHT,
        backgroundColor: "#f3f4f6",
        pointerEvents: "none",
        zIndex: -1,
      }}
    />
  );
}

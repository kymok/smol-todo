import { TITLE_BAR_HEIGHT } from "../../layout";

// Debug-only visualization of the title-bar / window-drag strip. Dragging
// itself is handled by startWindowDrag on the root, so this sits behind
// everything (zIndex: -1) and ignores pointer events. Remove once the title
// bar styling is finalized.
export function DebugTitleBarBackground() {
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

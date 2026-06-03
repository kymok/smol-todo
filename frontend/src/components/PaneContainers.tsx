import type { ReactNode } from "react";
import { TITLE_BAR_HEIGHT } from "../layout";

// Hardcoded 14px (intentionally NOT a spacing token) to match the native
// macOS sidebar inset.
const SIDEBAR_LEFT_INSET = 14;

// Outer sidebar shell: owns the resizable width and the positioning context for
// the right-edge resize handle, and applies the macOS window-chrome insets
// (title-bar top clearance + native left inset). Children include the sidebar
// content and the resize handle.
export function SidebarContainer({ width, children }: { width: number; children: ReactNode }) {
  return (
    <div
      style={{
        width,
        position: "relative",
        flexShrink: 0,
        paddingTop: TITLE_BAR_HEIGHT,
        paddingLeft: SIDEBAR_LEFT_INSET,
      }}
    >
      {children}
    </div>
  );
}

// Detail/content pane shell: fills the remaining width and clears the title bar.
export function DetailContainer({ children }: { children: ReactNode }) {
  return <div style={{ flexGrow: 1, paddingTop: TITLE_BAR_HEIGHT }}>{children}</div>;
}

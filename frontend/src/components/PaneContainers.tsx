import type { ReactNode } from "react";
import { TITLE_BAR_HEIGHT } from "../layout";

// Outer sidebar shell: owns the resizable width, the title-bar top clearance,
// and the positioning context for the right-edge resize handle. The inner
// wrapper applies the content padding (px-4). The handle is absolutely
// positioned against this outer div, so the inner wrapper doesn't affect it.
export function SidebarContainer({ width, children }: { width: number; children: ReactNode }) {
  return (
    <div
      className="relative flex h-full shrink-0 flex-col overflow-hidden"
      style={{ width, paddingTop: TITLE_BAR_HEIGHT }}
    >
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 py-0 select-none">{children}</div>
    </div>
  );
}

// Detail/content pane shell: fills the remaining width and applies the shared
// left inset so the title bar and content align. Title-bar clearance is owned by
// DetailPane's title row (which is TITLE_BAR_HEIGHT tall), so there is no top
// padding here.
export function DetailContainer({ children }: { children: ReactNode }) {
  return <div style={{ flexGrow: 1 }} className="flex h-full min-h-0 flex-col overflow-hidden px-4">{children}</div>;
}

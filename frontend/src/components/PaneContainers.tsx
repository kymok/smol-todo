import type { ReactNode } from "react";
import { TITLE_BAR_HEIGHT } from "../layout";

// Outer sidebar shell: owns the resizable width, the title-bar top clearance,
// and the positioning context for the right-edge resize handle. The inner
// wrapper applies the content padding (px-4). The handle is absolutely
// positioned against this outer div, so the inner wrapper doesn't affect it.
export function SidebarContainer({ width, children }: { width: number; children: ReactNode }) {
  return (
    <div style={{ width, position: "relative", flexShrink: 0, paddingTop: TITLE_BAR_HEIGHT }}>
      <div className="px-4 py-0">{children}</div>
    </div>
  );
}

// Detail/content pane shell: fills the remaining width and applies the shared
// left inset so the title bar and content align. Title-bar clearance is owned by
// DetailPane's title row (which is TITLE_BAR_HEIGHT tall), so there is no top
// padding here.
export function DetailContainer({ children }: { children: ReactNode }) {
  return <div style={{ flexGrow: 1 }} className={"px-4"}>{children}</div>;
}

import { useEffect, useState } from "react";
import { Flex } from "@radix-ui/themes";
import type { Snapshot } from "./api/types";
import { getSnapshot, onStoreChanged } from "./api/client";
import { ALL_COLLECTION, type ViewState } from "./state/view";
import { Sidebar } from "./components/Sidebar";
import { DetailPane } from "./components/DetailPane";

const EMPTY: Snapshot = { items: [], collections: [], groups: [] };

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY);
  const [view, setView] = useState<ViewState>({ selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: false, showArchived: false });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const refresh = () => { getSnapshot().then(setSnapshot).catch((e) => console.error(e)); };
    refresh();
    onStoreChanged(refresh).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return (
    <Flex height="100vh">
      <Sidebar snapshot={snapshot} selected={view.selected} onSelect={(name) => setView((v) => ({ ...v, selected: name }))} />
      <DetailPane snapshot={snapshot} view={view} onSearch={(q) => setView((v) => ({ ...v, search: q }))} />
    </Flex>
  );
}

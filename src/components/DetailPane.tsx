import { Flex, Heading, ScrollArea, TextField } from "@radix-ui/themes";
import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../state/view";
import { TaskRow } from "./TaskRow";

export function DetailPane({
  snapshot, view, onSearch,
}: { snapshot: Snapshot; view: ViewState; onSearch: (q: string) => void }) {
  const items = visibleItems(snapshot, view);
  const title = view.selected === ALL_COLLECTION
    ? "All"
    : snapshot.collections.find((c) => c.name === view.selected)?.displayName ?? view.selected;
  return (
    <Flex direction="column" flexGrow="1" p="3" gap="3">
      <Flex align="center" justify="between">
        <Heading size="4">{title}</Heading>
        <TextField.Root placeholder="Search" value={view.search} onChange={(e) => onSearch(e.target.value)}>
          <TextField.Slot><MagnifyingGlassIcon /></TextField.Slot>
        </TextField.Root>
      </Flex>
      <ScrollArea>
        <Flex direction="column">
          {items.map((item) => (
            <TaskRow key={item.id} item={item} showCollection={view.selected === ALL_COLLECTION} />
          ))}
        </Flex>
      </ScrollArea>
    </Flex>
  );
}

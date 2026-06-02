import { Badge, Box, Button, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, allIncompleteCount, sidebarGroups } from "../state/view";

export function Sidebar({
  snapshot, selected, onSelect,
}: { snapshot: Snapshot; selected: string; onSelect: (name: string) => void }) {
  return (
    <Flex direction="column" gap="1" p="2" style={{ width: 240 }}>
      <Button variant={selected === ALL_COLLECTION ? "soft" : "ghost"} onClick={() => onSelect(ALL_COLLECTION)}>
        <Flex align="center" gap="2" flexGrow="1">
          <Box flexGrow="1"><Text align="left">All</Text></Box>
          <Badge>{allIncompleteCount(snapshot)}</Badge>
        </Flex>
      </Button>
      {sidebarGroups(snapshot, false).map((group) => (
        <Box key={group.name} mt="2">
          <Text size="1" color="gray">{group.name === "DefaultGroup" ? "No Group" : group.name}</Text>
          {group.collections.map((c) => (
            <Button key={c.name} variant={selected === c.name ? "soft" : "ghost"} onClick={() => onSelect(c.name)}>
              <Flex align="center" gap="2" flexGrow="1">
                <Text color={c.color}><DotFilledIcon /></Text>
                <Box flexGrow="1"><Text align="left">{c.displayName}</Text></Box>
                <Badge>{c.incompleteCount}</Badge>
              </Flex>
            </Button>
          ))}
        </Box>
      ))}
    </Flex>
  );
}

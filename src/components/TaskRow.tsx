import { Badge, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
import type { CollectionColor, TaskItem, TaskStatus } from "../api/types";

const STATUS_COLOR: Record<TaskStatus, CollectionColor> = {
  draft: "gray", ready: "gray", "in-progress": "blue", completed: "green",
  "on-hold": "orange", rejected: "red", aborted: "red",
};

export function TaskRow({ item, showCollection }: { item: TaskItem; showCollection: boolean }) {
  const dim = item.status === "completed" || item.status === "in-progress";
  return (
    <Flex align="start" gap="2" py="1">
      <Text color={STATUS_COLOR[item.status]}><DotFilledIcon /></Text>
      <Flex direction="column" flexGrow="1">
        <Text size="2" color={dim ? "gray" : undefined}>{item.title || "Untitled"}</Text>
        {item.note ? <Text size="1" color="gray">{item.note.body}</Text> : null}
      </Flex>
      {showCollection ? <Badge color="gray" variant="soft">{item.collection}</Badge> : null}
    </Flex>
  );
}

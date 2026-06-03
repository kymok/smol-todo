import { useEffect, useState } from "react";
import { Dialog, Flex, Tabs, Text } from "@radix-ui/themes";
import { getVersion } from "@tauri-apps/api/app";
import { storePath } from "../api/client";

export interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const [version, setVersion] = useState<string>("");
  const [path, setPath] = useState<string>("");

  // Load version + store path when the dialog opens (cheap; re-fetch is fine).
  useEffect(() => {
    if (!open) return;
    getVersion()
      .then(setVersion)
      .catch((e) => console.error(e));
    storePath()
      .then(setPath)
      .catch((e) => console.error(e));
  }, [open]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content maxWidth="520px">
        <Dialog.Title>Settings</Dialog.Title>
        <Tabs.Root defaultValue="system">
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            {/* Prompt tab added in 5B; Command tab added in 5C. */}
          </Tabs.List>
          <Tabs.Content value="system">
            <Flex direction="column" gap="3" mt="3">
              <Flex justify="between" gap="4">
                <Text size="2" color="gray">Version</Text>
                <Text size="2">{version || "Unavailable"}</Text>
              </Flex>
              <Flex direction="column" gap="1">
                <Text size="2" color="gray">Store Path</Text>
                <Text size="1" style={{ wordBreak: "break-all" }}>{path || "Unavailable"}</Text>
              </Flex>
            </Flex>
          </Tabs.Content>
        </Tabs.Root>
      </Dialog.Content>
    </Dialog.Root>
  );
}

import { useEffect, useState } from "react";
import { Button, Dialog, Flex, Tabs, Text, TextArea } from "@radix-ui/themes";
import { getVersion } from "@tauri-apps/api/app";
import { storePath } from "../api/client";
import type { Settings } from "../api/types";

export interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: Settings;
  updateSettings: (patch: Partial<Settings>) => void;
}

export function SettingsDialog({ open, onOpenChange, settings, updateSettings }: SettingsDialogProps) {
  const [version, setVersion] = useState<string>("");
  const [path, setPath] = useState<string>("");
  const [promptDraft, setPromptDraft] = useState<string>("");

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

  // Seed the default-template editor from settings whenever the dialog opens.
  useEffect(() => {
    if (open) setPromptDraft(settings.defaultPromptTemplate);
  }, [open, settings.defaultPromptTemplate]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content maxWidth="520px">
        <Dialog.Title>Settings</Dialog.Title>
        <Tabs.Root defaultValue="system">
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            <Tabs.Trigger value="prompt">Prompt</Tabs.Trigger>
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
          <Tabs.Content value="prompt">
            <Flex direction="column" gap="2" mt="3">
              <Text size="2" color="gray">Default prompt template</Text>
              <TextArea
                value={promptDraft}
                onChange={(e) => setPromptDraft(e.target.value)}
                placeholder="Default prompt template…"
                rows={10}
              />
              <Text size="1" color="gray">
                Leave empty to use the built-in default. Collections without their own
                prompt use this template.
              </Text>
              <Flex justify="end">
                <Button onClick={() => updateSettings({ defaultPromptTemplate: promptDraft })}>
                  Save
                </Button>
              </Flex>
            </Flex>
          </Tabs.Content>
        </Tabs.Root>
      </Dialog.Content>
    </Dialog.Root>
  );
}

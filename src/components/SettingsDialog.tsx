import { useEffect, useState } from "react";
import { Button, Code, Dialog, Flex, IconButton, Link, Tabs, Text, TextArea } from "@radix-ui/themes";
import { CopyIcon } from "@radix-ui/react-icons";
import { getVersion } from "@tauri-apps/api/app";
import { cliInstall, cliInstallStatus, cliUninstall, storePath } from "../api/client";
import { copyText } from "../lib/clipboard";
import type { InstallStatus, Settings } from "../api/types";

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
  const [installStatus, setInstallStatus] = useState<InstallStatus | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);

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

  // Fetch CLI-install status whenever the dialog opens (the Command tab reads it).
  useEffect(() => {
    if (!open) return;
    cliInstallStatus()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  }, [open]);

  const runInstall = () => {
    cliInstall()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  };
  const runUninstall = () => {
    cliUninstall()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  };
  const statusText = (s: InstallStatus): string =>
    s.installed ? "Installed" : (s.conflictDescription ?? "Not installed");

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content maxWidth="520px">
        <Dialog.Title>Settings</Dialog.Title>
        <Tabs.Root defaultValue="system">
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            <Tabs.Trigger value="prompt">Prompt</Tabs.Trigger>
            <Tabs.Trigger value="command">Command</Tabs.Trigger>
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
          <Tabs.Content value="command">
            <Flex direction="column" gap="3" mt="3">
              {installStatus ? (
                <>
                  <Flex direction="column" gap="1">
                    <Text size="2" color="gray">Link</Text>
                    <Link size="1" style={{ wordBreak: "break-all" }}>{installStatus.linkPath}</Link>
                  </Flex>
                  <Flex direction="column" gap="1">
                    <Text size="2" color="gray">Status</Text>
                    <Text size="2" color={installStatus.installed ? "green" : "gray"}>
                      {statusText(installStatus)}
                    </Text>
                  </Flex>
                  {!installStatus.installDirectoryIsInPath && (
                    <Flex direction="column" gap="1">
                      <Text size="2" color="gray">Add to PATH</Text>
                      <Flex align="center" gap="2">
                        <Code size="1" style={{ wordBreak: "break-all" }}>{installStatus.pathHint}</Code>
                        <IconButton
                          size="1"
                          variant="soft"
                          aria-label="Copy PATH command"
                          onClick={() => copyText(installStatus.pathHint).catch((e) => setInstallError(String(e)))}
                        >
                          <CopyIcon />
                        </IconButton>
                      </Flex>
                    </Flex>
                  )}
                  {installError && <Text size="1" color="red">{installError}</Text>}
                  <Flex gap="2" justify="end">
                    <Button
                      color="red"
                      variant="soft"
                      disabled={!installStatus.canUninstall}
                      onClick={runUninstall}
                    >
                      Uninstall
                    </Button>
                    <Button
                      disabled={!installStatus.installed && !installStatus.canInstall}
                      onClick={runInstall}
                    >
                      {installStatus.installed ? "Reinstall" : "Install"}
                    </Button>
                  </Flex>
                </>
              ) : (
                <Text size="2" color={installError ? "red" : "gray"}>
                  {installError ?? "Loading…"}
                </Text>
              )}
            </Flex>
          </Tabs.Content>
        </Tabs.Root>
      </Dialog.Content>
    </Dialog.Root>
  );
}

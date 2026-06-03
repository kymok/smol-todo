import { useEffect, useState } from "react";
import { Dialog } from "@base-ui-components/react/dialog";
import { Tabs } from "@base-ui-components/react/tabs";
import { Copy } from "lucide-react";
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
      <Dialog.Portal>
        <Dialog.Backdrop />
        <Dialog.Popup>
          <Dialog.Title>Settings</Dialog.Title>
          <Tabs.Root defaultValue="system">
            <Tabs.List>
              <Tabs.Tab value="system">System Information</Tabs.Tab>
              <Tabs.Tab value="prompt">Prompt</Tabs.Tab>
              <Tabs.Tab value="command">Command</Tabs.Tab>
            </Tabs.List>
            <Tabs.Panel value="system">
              <div>
                <div>
                  <span>Version</span>
                  <span>{version || "Unavailable"}</span>
                </div>
                <div>
                  <span>Store Path</span>
                  <span style={{ wordBreak: "break-all" }}>{path || "Unavailable"}</span>
                </div>
              </div>
            </Tabs.Panel>
            <Tabs.Panel value="prompt">
              <div>
                <span>Default prompt template</span>
                <textarea
                  value={promptDraft}
                  onChange={(e) => setPromptDraft(e.target.value)}
                  placeholder="Default prompt template…"
                  rows={10}
                />
                <span>
                  Leave empty to use the built-in default. Collections without their own
                  prompt use this template.
                </span>
                <div>
                  <button onClick={() => updateSettings({ defaultPromptTemplate: promptDraft })}>
                    Save
                  </button>
                </div>
              </div>
            </Tabs.Panel>
            <Tabs.Panel value="command">
              <div>
                {installStatus ? (
                  <>
                    <div>
                      <span>Link</span>
                      <a style={{ wordBreak: "break-all" }}>{installStatus.linkPath}</a>
                    </div>
                    <div>
                      <span>Status</span>
                      <span>{statusText(installStatus)}</span>
                    </div>
                    {!installStatus.installDirectoryIsInPath && (
                      <div>
                        <span>Add to PATH</span>
                        <div>
                          <code style={{ wordBreak: "break-all" }}>{installStatus.pathHint}</code>
                          <button
                            aria-label="Copy PATH command"
                            onClick={() => copyText(installStatus.pathHint).catch((e) => setInstallError(String(e)))}
                          >
                            <Copy />
                          </button>
                        </div>
                      </div>
                    )}
                    {installError && <span>{installError}</span>}
                    <div>
                      <button
                        disabled={!installStatus.canUninstall}
                        onClick={runUninstall}
                      >
                        Uninstall
                      </button>
                      <button
                        disabled={!installStatus.installed && !installStatus.canInstall}
                        onClick={runInstall}
                      >
                        {installStatus.installed ? "Reinstall" : "Install"}
                      </button>
                    </div>
                  </>
                ) : (
                  <span>{installError ?? "Loading…"}</span>
                )}
              </div>
            </Tabs.Panel>
          </Tabs.Root>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

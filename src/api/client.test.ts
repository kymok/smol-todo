import { describe, expect, it, vi, beforeEach } from "vitest";
import type { TaskItem } from "./types";

const invokeMock = vi.fn();
const listenMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listenMock(...a) }));

import { getSnapshot, onStoreChanged, createItem, setStatus, getSettings, setSettings } from "./client";

describe("api client", () => {
  beforeEach(() => { invokeMock.mockReset(); listenMock.mockReset(); });

  it("getSnapshot invokes the get_snapshot command", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    const snap = await getSnapshot();
    expect(invokeMock).toHaveBeenCalledWith("get_snapshot");
    expect(snap.items).toEqual([]);
  });

  it("onStoreChanged registers a store-changed listener", async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);
    const cb = vi.fn();
    await onStoreChanged(cb);
    expect(listenMock).toHaveBeenCalledWith("store-changed", expect.any(Function));
  });

  it("createItem invokes create_item with the collection arg", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await createItem("Work/Docs");
    expect(invokeMock).toHaveBeenCalledWith("create_item", { collection: "Work/Docs" });
  });

  it("setStatus invokes set_status with id/status/ifCurrent", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    const item = { id: "00000001" } as unknown as TaskItem;
    await setStatus("completed", "00000001", item);
    expect(invokeMock).toHaveBeenCalledWith("set_status", {
      status: "completed",
      id: "00000001",
      ifCurrent: item,
    });
  });

  it("getSettings invokes get_settings with no args", async () => {
    const settings = {
      usesAutoDraft: true,
      alwaysOnTop: false,
      defaultPromptTemplate: "",
      lastSelectedCollection: null,
    };
    invokeMock.mockResolvedValue(settings);
    await expect(getSettings()).resolves.toEqual(settings);
    expect(invokeMock).toHaveBeenCalledWith("get_settings");
  });

  it("setSettings invokes set_settings with the whole settings object", async () => {
    const settings = {
      usesAutoDraft: false,
      alwaysOnTop: true,
      defaultPromptTemplate: "",
      lastSelectedCollection: "Work/Docs",
    };
    invokeMock.mockResolvedValue(settings);
    await setSettings(settings);
    expect(invokeMock).toHaveBeenCalledWith("set_settings", { settings });
  });
});

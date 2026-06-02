import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listenMock(...a) }));

import { getSnapshot, onStoreChanged } from "./client";

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
});

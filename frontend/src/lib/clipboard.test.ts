import { describe, expect, it, vi, beforeEach } from "vitest";

const writeTextMock = vi.fn();
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: (...a: unknown[]) => writeTextMock(...a),
}));

import { copyText } from "./clipboard";

describe("clipboard", () => {
  beforeEach(() => writeTextMock.mockReset());

  it("copyText calls the plugin writeText with the given string", async () => {
    writeTextMock.mockResolvedValue(undefined);
    await copyText("hello");
    expect(writeTextMock).toHaveBeenCalledWith("hello");
  });
});

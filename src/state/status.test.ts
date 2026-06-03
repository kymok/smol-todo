import { describe, expect, it } from "vitest";
import { leadingStatusClickTarget, rightClickStatusTarget } from "./status";

describe("leadingStatusClickTarget", () => {
  it("ready and in-progress advance to completed", () => {
    expect(leadingStatusClickTarget("ready")).toBe("completed");
    expect(leadingStatusClickTarget("in-progress")).toBe("completed");
  });
  it("everything else advances to ready", () => {
    expect(leadingStatusClickTarget("draft")).toBe("ready");
    expect(leadingStatusClickTarget("completed")).toBe("ready");
    expect(leadingStatusClickTarget("on-hold")).toBe("ready");
    expect(leadingStatusClickTarget("rejected")).toBe("ready");
    expect(leadingStatusClickTarget("aborted")).toBe("ready");
  });
  it("right-click always targets draft", () => {
    expect(rightClickStatusTarget("ready")).toBe("draft");
    expect(rightClickStatusTarget("completed")).toBe("draft");
  });
});

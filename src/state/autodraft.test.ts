import { describe, expect, it } from "vitest";
import { autoDraftStatus } from "./autodraft";

describe("autoDraftStatus — confirm phase", () => {
  it("a draft always promotes to ready on confirm (even with auto-draft off)", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "draft", phase: "confirm", titleChanged: false }),
    ).toBe("ready");
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "draft", phase: "confirm", titleChanged: true }),
    ).toBe("ready");
  });

  it("a non-draft promotes to ready on confirm only when auto-draft is on", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "on-hold", phase: "confirm", titleChanged: true }),
    ).toBe("ready");
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "on-hold", phase: "confirm", titleChanged: true }),
    ).toBeUndefined();
  });
});

describe("autoDraftStatus — edit phase", () => {
  it("no title change → no status change", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "ready", phase: "edit", titleChanged: false }),
    ).toBeUndefined();
  });

  it("a draft stays draft while editing (no status change)", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "draft", phase: "edit", titleChanged: true }),
    ).toBeUndefined();
  });

  it("a non-draft drops to draft on edit only when auto-draft is on", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "ready", phase: "edit", titleChanged: true }),
    ).toBe("draft");
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "ready", phase: "edit", titleChanged: true }),
    ).toBeUndefined();
  });
});

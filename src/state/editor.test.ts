import { describe, expect, it } from "vitest";
import { reduceKey, type KeyContext } from "./editor";

function ctx(over: Partial<KeyContext>): KeyContext {
  return {
    key: "Enter",
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ctrlKey: false,
    caretAtStart: false,
    caretAtEnd: false,
    value: "hello",
    field: "title",
    composing: false,
    ...over,
  };
}

describe("editor reducer — title field", () => {
  it("Enter splits at caret", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "ab", caretAtStart: false, caretAtEnd: false })))
      .toEqual({ type: "Split" });
  });

  it("Enter at end of a non-empty title creates a task below (Split with caretAtEnd)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "ab", caretAtEnd: true })))
      .toEqual({ type: "Split" });
  });

  it("Enter on an empty title commits (which deletes the empty draft)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "", caretAtStart: true, caretAtEnd: true })))
      .toEqual({ type: "DeleteEmpty" });
  });

  it("Enter while composing is suppressed (IME)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "あ", composing: true })))
      .toEqual({ type: "None" });
  });

  it("Cmd+Enter commits a non-empty title", () => {
    expect(reduceKey(ctx({ key: "Enter", metaKey: true, value: "ab" })))
      .toEqual({ type: "Commit" });
  });

  it("Cmd+Enter on an empty title deletes", () => {
    expect(reduceKey(ctx({ key: "Enter", metaKey: true, value: "" })))
      .toEqual({ type: "DeleteEmpty" });
  });

  it("Backspace at caret 0 merges into previous", () => {
    expect(reduceKey(ctx({ key: "Backspace", caretAtStart: true, value: "x" })))
      .toEqual({ type: "MergeIntoPrevious" });
  });

  it("Backspace not at start is a no-op (let the textarea edit)", () => {
    expect(reduceKey(ctx({ key: "Backspace", caretAtStart: false, value: "x" })))
      .toEqual({ type: "None" });
  });

  it("Tab commits and moves focus down", () => {
    expect(reduceKey(ctx({ key: "Tab", value: "ab" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("Tab on empty deletes then moves down", () => {
    expect(reduceKey(ctx({ key: "Tab", value: "" })))
      .toEqual({ type: "DeleteEmpty", thenFocus: "down" });
  });

  it("Escape discards", () => {
    expect(reduceKey(ctx({ key: "Escape" }))).toEqual({ type: "Discard" });
  });

  it("ArrowUp / ArrowDown move focus", () => {
    expect(reduceKey(ctx({ key: "ArrowUp" }))).toEqual({ type: "MoveFocus", dir: "up" });
    expect(reduceKey(ctx({ key: "ArrowDown" }))).toEqual({ type: "MoveFocus", dir: "down" });
  });

  it("any other key is None (textarea handles it)", () => {
    expect(reduceKey(ctx({ key: "a" }))).toEqual({ type: "None" });
  });

  it("Shift+Enter does NOT split (let textarea insert newline)", () => {
    expect(reduceKey(ctx({ key: "Enter", shiftKey: true, value: "ab" })))
      .toEqual({ type: "None" });
  });

  it("Cmd+Backspace at caret 0 does NOT merge (delete-word shortcut)", () => {
    expect(reduceKey(ctx({ key: "Backspace", metaKey: true, caretAtStart: true, value: "x" })))
      .toEqual({ type: "None" });
  });

  it("Alt+Backspace at caret 0 does NOT merge (Option+Backspace = delete word)", () => {
    expect(reduceKey(ctx({ key: "Backspace", altKey: true, caretAtStart: true, value: "x" })))
      .toEqual({ type: "None" });
  });

  it("Shift+Backspace at caret 0 still merges (Shift is not a modifier guard)", () => {
    expect(reduceKey(ctx({ key: "Backspace", shiftKey: true, caretAtStart: true, value: "x" })))
      .toEqual({ type: "MergeIntoPrevious" });
  });

  it("Tab while composing is suppressed (IME)", () => {
    expect(reduceKey(ctx({ key: "Tab", composing: true, value: "か" })))
      .toEqual({ type: "None" });
  });
});

describe("editor reducer — note field", () => {
  it("Enter (Return) moves focus down", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "n" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("Tab moves focus down", () => {
    expect(reduceKey(ctx({ field: "note", key: "Tab", value: "n" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("empty note on commit deletes the note", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "" })))
      .toEqual({ type: "DeleteEmpty", thenFocus: "down" });
  });

  it("Escape discards", () => {
    expect(reduceKey(ctx({ field: "note", key: "Escape" }))).toEqual({ type: "Discard" });
  });

  it("Enter while composing is suppressed (IME)", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "ん", composing: true })))
      .toEqual({ type: "None" });
  });
});

import { describe, expect, it } from "vitest";

import type { LiveConversationItem } from "../../app-server/event-reducer";
import {
  buildUserTextInput,
  canSteerTurn,
  mergeConversationItems,
} from "./conversation-model";

describe("conversation workspace selectors", () => {
  it("renders authoritative transcript items and replaces a live streaming item by id", () => {
    const authoritative = [
      {
        turnId: "turn-1",
        item: { type: "agentMessage", id: "item-1", text: "Hello" },
      },
    ] as never;
    const live: LiveConversationItem[] = [
      {
        id: "item-1",
        threadId: "thread-1",
        turnId: "turn-1",
        type: "agentMessage",
        text: "Hello world",
      },
      {
        id: "item-2",
        threadId: "thread-1",
        turnId: "turn-1",
        type: "agentMessage",
        text: "Next item",
      },
    ];

    expect(mergeConversationItems(authoritative, live)).toEqual([
      {
        id: "item-1",
        turnId: "turn-1",
        type: "agentMessage",
        text: "Hello world",
      },
      {
        id: "item-2",
        turnId: "turn-1",
        type: "agentMessage",
        text: "Next item",
      },
    ]);
  });

  it("only exposes Stop/Steer actions when a turn is active", () => {
    expect(canSteerTurn("turn-1")).toBe(true);
    expect(canSteerTurn(null)).toBe(false);
  });

  it("builds the protocol text input used by normal send and steer", () => {
    expect(buildUserTextInput("Inspect this project")).toEqual({
      type: "text",
      text: "Inspect this project",
      text_elements: [],
    });
  });
});

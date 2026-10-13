import type { v2 } from "../../protocol";
import type { LiveConversationItem } from "../../app-server/event-reducer";

export type ConversationViewItem = {
  id: string;
  turnId: string;
  type: string;
  text: string;
};

export function mergeConversationItems(
  authoritative: v2.ThreadItemsListResponse["data"],
  live: LiveConversationItem[],
): ConversationViewItem[] {
  const liveById = new Map(live.map((item) => [item.id, item]));
  const seen = new Set<string>();
  const merged = authoritative.map(({ turnId, item }) => {
    seen.add(item.id);
    const liveItem = liveById.get(item.id);
    return liveItem
      ? { id: liveItem.id, turnId: liveItem.turnId, type: liveItem.type, text: liveItem.text }
      : threadItemToViewItem(turnId, item);
  });

  for (const item of live) {
    if (!seen.has(item.id)) {
      merged.push({ id: item.id, turnId: item.turnId, type: item.type, text: item.text });
    }
  }
  return merged;
}

export function buildUserTextInput(text: string): v2.UserInput {
  return { type: "text", text, text_elements: [] };
}

export function canSteerTurn(activeTurnId: string | null): boolean {
  return Boolean(activeTurnId);
}

function threadItemToViewItem(turnId: string, item: v2.ThreadItem): ConversationViewItem {
  switch (item.type) {
    case "userMessage":
      return {
        id: item.id,
        turnId,
        type: item.type,
        text: item.content
          .filter((content) => content.type === "text")
          .map((content) => content.text)
          .join("\n"),
      };
    case "agentMessage":
    case "plan":
      return { id: item.id, turnId, type: item.type, text: item.text };
    case "reasoning":
      return {
        id: item.id,
        turnId,
        type: item.type,
        text: [...item.summary, ...item.content].join("\n"),
      };
    default:
      return { id: item.id, turnId, type: item.type, text: JSON.stringify(item) };
  }
}

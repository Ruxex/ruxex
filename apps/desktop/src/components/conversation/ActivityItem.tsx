import type { ConversationViewItem } from "./conversation-model";

export function ActivityItem({ item }: { item: ConversationViewItem }) {
  return (
    <details className="collapse collapse-arrow border border-base-300 bg-base-100">
      <summary className="collapse-title py-3 text-sm font-medium">
        {item.type === "plan" ? "Plan" : item.type === "reasoning" ? "Reasoning" : item.type}
      </summary>
      <div className="collapse-content">
        <p className="whitespace-pre-wrap text-sm text-base-content/80">{item.text || "…"}</p>
      </div>
    </details>
  );
}

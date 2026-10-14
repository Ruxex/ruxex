import type { ReactNode } from "react";

export function LoadingState({ size = "lg" }: { size?: "sm" | "lg" }) {
  return <span className={`loading loading-spinner loading-${size}`} />;
}

export function ErrorState({ children }: { children: ReactNode }) {
  return <p className="alert alert-error">{children}</p>;
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <p className="text-base-content/60">{children}</p>;
}

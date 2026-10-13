import type { ReactNode } from "react";

type ConnectionCardProps = {
  connected: boolean;
  appServerVersion?: string;
  resynchronizing: boolean;
  error?: unknown;
  isFetching: boolean;
  onReconnect: () => void;
};

export function ConnectionCard({
  connected,
  appServerVersion,
  resynchronizing,
  error,
  isFetching,
  onReconnect,
}: ConnectionCardProps) {
  return (
    <section className="card border border-base-300 bg-base-100 shadow-sm">
      <div className="card-body">
        <h2 className="card-title">Connection</h2>
        <dl className="grid gap-2 text-sm">
          <SettingRow label="Status">{connected ? "Connected" : "Disconnected"}</SettingRow>
          <SettingRow label="App-server">{appServerVersion ?? "Unavailable"}</SettingRow>
          <SettingRow label="Live state">
            {resynchronizing ? "Resynchronizing" : "Synchronized"}
          </SettingRow>
        </dl>
        {error !== undefined && error !== null && (
          <p className="mt-2 text-sm text-error">{String(error)}</p>
        )}
        <button
          className="btn btn-sm self-end"
          type="button"
          onClick={onReconnect}
          disabled={isFetching}
        >
          {isFetching ? "Reconnecting…" : "Reconnect"}
        </button>
      </div>
    </section>
  );
}

type ProviderAccountCardProps = {
  provider: string;
  authStatus: string;
  account: string;
};

export function ProviderAccountCard({
  provider,
  authStatus,
  account,
}: ProviderAccountCardProps) {
  return (
    <section className="card border border-base-300 bg-base-100 shadow-sm">
      <div className="card-body">
        <h2 className="card-title">Provider and account</h2>
        <dl className="grid gap-2 text-sm">
          <SettingRow label="Provider">{provider}</SettingRow>
          <SettingRow label="Auth">{authStatus}</SettingRow>
          <SettingRow label="Account">{account}</SettingRow>
        </dl>
      </div>
    </section>
  );
}

export function CapabilitiesCard({
  capabilities,
}: {
  capabilities: Record<string, boolean | string>;
}) {
  return (
    <section className="card border border-base-300 bg-base-100 shadow-sm">
      <div className="card-body">
        <h2 className="card-title">Capabilities</h2>
        <div className="grid gap-2 text-sm sm:grid-cols-2 lg:grid-cols-3">
          {Object.entries(capabilities).map(([name, enabled]) => (
            <div className="flex justify-between gap-3" key={name}>
              <span>{name}</span>
              <span className={enabled ? "text-success" : "text-base-content/50"}>
                {String(enabled)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

type RuntimeSummaryProps = {
  modelCount: number;
  permissionCount: number;
  rateLimitName: string;
};

export function RuntimeSummary({
  modelCount,
  permissionCount,
  rateLimitName,
}: RuntimeSummaryProps) {
  return (
    <section className="grid gap-4 md:grid-cols-3">
      <SummaryCard label="Models">{modelCount} available</SummaryCard>
      <SummaryCard label="Permissions">{permissionCount} profiles</SummaryCard>
      <SummaryCard label="Rate limits">{rateLimitName}</SummaryCard>
    </section>
  );
}

function SettingRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex justify-between gap-3">
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

function SummaryCard({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="card border border-base-300 bg-base-100 shadow-sm">
      <div className="card-body">
        <h2 className="card-title text-base">{label}</h2>
        <p>{children}</p>
      </div>
    </div>
  );
}

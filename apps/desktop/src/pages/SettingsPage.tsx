import {
  useAccount,
  useAppServerEvents,
  useAppServerStatus,
  useModelList,
  useModelProvider,
  useModelProviderCapabilities,
  usePermissionProfiles,
} from "../app-server/hooks";

export function SettingsPage() {
  const status = useAppServerStatus();
  const events = useAppServerEvents();
  const account = useAccount();
  const provider = useModelProvider();
  const capabilities = useModelProviderCapabilities();
  const models = useModelList();
  const permissions = usePermissionProfiles();

  return (
    <div className="space-y-5">
      <div>
        <p className="text-sm font-medium uppercase tracking-wide text-primary">Runtime</p>
        <h1 className="text-3xl font-semibold">Settings</h1>
      </div>
      <section className="grid gap-4 md:grid-cols-2">
        <div className="card border border-base-300 bg-base-100 shadow-sm">
          <div className="card-body">
            <h2 className="card-title">Connection</h2>
            <dl className="grid gap-2 text-sm">
              <div className="flex justify-between gap-3"><dt>Status</dt><dd>{events.connected ? "Connected" : "Disconnected"}</dd></div>
              <div className="flex justify-between gap-3"><dt>App-server</dt><dd>{status.data?.appServerVersion ?? "Unavailable"}</dd></div>
              <div className="flex justify-between gap-3"><dt>Live state</dt><dd>{events.resynchronizing ? "Resynchronizing" : "Synchronized"}</dd></div>
            </dl>
            {status.isError && <p className="mt-2 text-sm text-error">{String(status.error)}</p>}
            <button className="btn btn-sm self-end" type="button" onClick={() => void status.refetch()} disabled={status.isFetching}>
              {status.isFetching ? "Reconnecting…" : "Reconnect"}
            </button>
          </div>
        </div>
        <div className="card border border-base-300 bg-base-100 shadow-sm">
          <div className="card-body">
            <h2 className="card-title">Provider and account</h2>
            <dl className="grid gap-2 text-sm">
              <div className="flex justify-between gap-3"><dt>Provider</dt><dd>{provider.data?.name ?? account.data?.providerId ?? "Unavailable"}</dd></div>
              <div className="flex justify-between gap-3"><dt>Auth</dt><dd>{account.data?.authStatus ?? "Unknown"}</dd></div>
              <div className="flex justify-between gap-3"><dt>Account</dt><dd>{account.data?.displayName ?? "Not signed in"}</dd></div>
            </dl>
          </div>
        </div>
      </section>
      <section className="card border border-base-300 bg-base-100 shadow-sm">
        <div className="card-body">
          <h2 className="card-title">Capabilities</h2>
          <div className="grid gap-2 text-sm sm:grid-cols-2 lg:grid-cols-3">
            {capabilities.data && Object.entries(capabilities.data).map(([name, enabled]) => (
              <div className="flex justify-between gap-3" key={name}>
                <span>{name}</span><span className={enabled ? "text-success" : "text-base-content/50"}>{String(enabled)}</span>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="grid gap-4 md:grid-cols-3">
        <div className="card border border-base-300 bg-base-100 shadow-sm"><div className="card-body"><h2 className="card-title text-base">Models</h2><p>{models.data?.data.length ?? 0} available</p></div></div>
        <div className="card border border-base-300 bg-base-100 shadow-sm"><div className="card-body"><h2 className="card-title text-base">Permissions</h2><p>{permissions.data?.data.length ?? 0} profiles</p></div></div>
        <div className="card border border-base-300 bg-base-100 shadow-sm"><div className="card-body"><h2 className="card-title text-base">Rate limits</h2><p>{account.data?.usage?.limitName ?? "Unavailable"}</p></div></div>
      </section>
    </div>
  );
}

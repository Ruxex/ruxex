import {
  useAccount,
  useAppServerEvents,
  useAppServerStatus,
  useModelList,
  useModelProvider,
  useModelProviderCapabilities,
  usePermissionProfiles,
} from "../app-server/hooks";
import {
  CapabilitiesCard,
  ConnectionCard,
  ProviderAccountCard,
  RuntimeSummary,
} from "../components/settings/SettingsCards";

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
        <ConnectionCard
          connected={events.connected}
          appServerVersion={status.data?.appServerVersion}
          resynchronizing={events.resynchronizing}
          error={status.isError ? status.error : undefined}
          isFetching={status.isFetching}
          onReconnect={() => void status.refetch()}
        />
        <ProviderAccountCard
          provider={provider.data?.name ?? account.data?.providerId ?? "Unavailable"}
          authStatus={account.data?.authStatus ?? "Unknown"}
          account={account.data?.displayName ?? "Not signed in"}
        />
      </section>
      <CapabilitiesCard capabilities={capabilities.data ?? {}} />
      <RuntimeSummary
        modelCount={models.data?.data.length ?? 0}
        permissionCount={permissions.data?.data.length ?? 0}
        rateLimitName={account.data?.usage?.limitName ?? "Unavailable"}
      />
    </div>
  );
}

import { Link } from "react-router-dom";

import { useAccount } from "../app-server/hooks";

export function AccountPage() {
  const account = useAccount();

  if (account.isLoading) {
    return <span className="loading loading-spinner loading-lg" />;
  }

  if (account.isError) {
    return <p className="alert alert-error">{String(account.error)}</p>;
  }

  const data = account.data;

  return (
    <div className="mx-auto max-w-2xl">
      <div className="card border border-base-300 bg-base-100 shadow-sm">
        <div className="card-body gap-5">
          <div>
            <p className="text-sm font-medium uppercase tracking-wide text-primary">
              Account
            </p>
            <h1 className="card-title text-3xl">Account information</h1>
          </div>
          <div className="rounded-box bg-base-200 p-4">
            <dl className="grid gap-3 sm:grid-cols-2">
              <div>
                <dt className="text-sm text-base-content/60">Status</dt>
                <dd className="font-medium">
                  {data?.authStatus === "authenticated"
                    ? "Authenticated"
                    : "Authentication required"}
                </dd>
              </div>
              <div>
                <dt className="text-sm text-base-content/60">Account type</dt>
                <dd className="font-medium">
                  {data?.account?.type ?? "Not signed in"}
                </dd>
              </div>
            </dl>
          </div>
          {data?.authStatus !== "authenticated" && (
            <Link className="btn btn-primary self-end" to="/login">
              Log in
            </Link>
          )}
        </div>
      </div>
    </div>
  );
}

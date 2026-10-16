import { Link, NavLink, useNavigate } from "react-router-dom";

import {
  useAccount,
  useAppServerEvents,
  useAppServerStatus,
  useLogout,
} from "../../app-server/hooks";

function useConnectionLabel() {
  const appServer = useAppServerStatus();
  const events = useAppServerEvents();

  const label = appServer.isError
    ? "Error"
    : !appServer.isSuccess
      ? "Local"
      : events.connected
        ? "Connected"
        : "Disconnected";

  return label;
}

export function TopBar() {
  const navigate = useNavigate();
  const account = useAccount();
  const logout = useLogout();
  const connectionLabel = useConnectionLabel();

  return (
    <div className="navbar flex-wrap gap-2 border-b border-base-300 bg-base-100 px-6">
      <div className="flex-1">
        <Link className="text-xl font-semibold" to="/">
          Workspace
        </Link>
      </div>
      <nav className="flex items-center gap-1" aria-label="Main navigation">
        <NavLink className="btn btn-ghost btn-sm" to="/threads">
          Threads
        </NavLink>
        <NavLink className="btn btn-ghost btn-sm" to="/chat/new">
          New chat
        </NavLink>
        <NavLink className="btn btn-ghost btn-sm" to="/account">
          Account
        </NavLink>
        <NavLink className="btn btn-ghost btn-sm" to="/approvals">
          Approvals
        </NavLink>
        <NavLink className="btn btn-ghost btn-sm" to="/settings">
          Settings
        </NavLink>
      </nav>
      <div className="badge badge-outline gap-2">
        <span
          className={`status ${connectionLabel === "Connected" ? "status-success" : "status-warning"}`}
          aria-hidden="true"
        />
        {connectionLabel}
      </div>
      {account.data?.account && (
        <button
          className="btn btn-ghost btn-sm"
          type="button"
          disabled={logout.isPending}
          onClick={() => {
            void logout
              .mutateAsync()
              .then(() => navigate("/login"))
              .catch(() => undefined);
          }}
        >
          {logout.isPending ? "Logging out…" : "Log out"}
        </button>
      )}
    </div>
  );
}

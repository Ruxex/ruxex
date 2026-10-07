import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useAccount, useLogin } from "../app-server/hooks";

export function LoginPage() {
  const navigate = useNavigate();
  const account = useAccount();
  const login = useLogin();
  const [apiKey, setApiKey] = useState("");

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await login.mutateAsync({ type: "apiKey", apiKey });
      await account.refetch();
      navigate("/account");
    } catch {
      // The mutation state renders the error next to the form.
    }
  }

  return (
    <div className="mx-auto max-w-lg">
      <div className="card border border-base-300 bg-base-100 shadow-sm">
        <form
          className="card-body gap-5"
          onSubmit={(event) => void handleSubmit(event)}
        >
          <div>
            <p className="text-sm font-medium uppercase tracking-wide text-primary">
              Account
            </p>
            <h1 className="card-title text-3xl">Log in</h1>
            <p className="mt-2 text-base-content/70">
              Connect this local desktop app with an API key.
            </p>
          </div>
          <label className="form-control">
            <span className="label-text">API key</span>
            <input
              className="input input-bordered mt-2"
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder="Enter your API key"
              required
            />
          </label>
          {login.isError && <p className="text-error">{String(login.error)}</p>}
          <div className="card-actions items-center justify-between">
            <Link className="link link-hover" to="/">
              Back home
            </Link>
            <button
              className="btn btn-primary"
              type="submit"
              disabled={login.isPending}
            >
              {login.isPending ? "Logging in…" : "Log in"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

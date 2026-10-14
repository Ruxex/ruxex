import { beforeEach, describe, expect, it, vi } from "vitest";

import { requestAppServerMethod } from "./api";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

describe("requestAppServerMethod", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue({
      method: "thread/list",
      id: 1,
      response: { data: [], nextCursor: null, backwardsCursor: null },
    });
  });

  it("builds a typed app-server request envelope", async () => {
    await requestAppServerMethod("thread/list", { limit: 25 });

    expect(invoke).toHaveBeenCalledWith("app_server_request", {
      request: {
        method: "thread/list",
        id: 1,
        params: { limit: 25 },
      },
    });
  });
});

import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ConnectionError,
  ProviderConnection,
  ProviderKeyTestOutcome,
  RemoveProviderKeyArgs,
  SaveProviderKeyArgs,
  TestProviderKeyArgs,
} from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

// Import after mock is registered so bindings.ts picks up the mock.
const { connectionGateway } = await import("./gateway");

const makeConnection = (): ProviderConnection => ({
  provider: "Stooq",
  has_key: true,
  active_tier: "OsKeychain",
});

describe("connection gateway — getProviderConnections", () => {
  beforeEach(() => vi.clearAllMocks());

  // get_provider_connections — ok pass-through (KEY-016)
  it("getProviderConnections passes through ok result with connection list", async () => {
    const connections: ProviderConnection[] = [makeConnection()];
    mockInvoke.mockResolvedValue(connections);

    const result = await connectionGateway.getProviderConnections();

    expect(result).toEqual({ status: "ok", data: connections });
    expect(mockInvoke).toHaveBeenCalledWith("get_provider_connections");
  });

  // get_provider_connections — empty list is a valid success
  it("getProviderConnections passes through ok result with empty list", async () => {
    mockInvoke.mockResolvedValue([]);

    const result = await connectionGateway.getProviderConnections();

    expect(result).toEqual({ status: "ok", data: [] });
    expect(mockInvoke).toHaveBeenCalledWith("get_provider_connections");
  });

  // get_provider_connections — KeyStoreError pass-through (F27: gateway does NOT throw)
  it("getProviderConnections passes through KeyStoreError result", async () => {
    const err: ConnectionError = { code: "KeyStoreError" };
    mockInvoke.mockRejectedValue(err);

    const result = await connectionGateway.getProviderConnections();

    expect(result).toEqual({ status: "error", error: err });
    expect(result.status).toBe("error");
    if (result.status === "error") expect(result.error.code).toBe("KeyStoreError");
  });

  // has_key: false, active_tier: null — no-key connection shape
  it("getProviderConnections passes through connection with has_key false and null active_tier", async () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: false, active_tier: null },
    ];
    mockInvoke.mockResolvedValue(connections);

    const result = await connectionGateway.getProviderConnections();

    expect(result).toEqual({ status: "ok", data: connections });
    if (result.status === "ok") {
      expect(result.data[0]?.has_key).toBe(false);
      expect(result.data[0]?.active_tier).toBeNull();
    }
  });
});

describe("connection gateway — saveProviderKey", () => {
  beforeEach(() => vi.clearAllMocks());

  // save_provider_key — ok pass-through, positional args (KEY-010/011)
  it("saveProviderKey passes through ok result with resulting ProviderConnection", async () => {
    const connection = makeConnection();
    mockInvoke.mockResolvedValue(connection);

    const args: SaveProviderKeyArgs = {
      provider: "Stooq",
      key: "my-secret-key",
      allow_plaintext: false,
    };
    const result = await connectionGateway.saveProviderKey(args);

    expect(result).toEqual({ status: "ok", data: connection });
    expect(mockInvoke).toHaveBeenCalledWith("save_provider_key", { args });
  });

  // save_provider_key — returns connection showing active_tier after save
  it("saveProviderKey returns connection with active_tier from the result", async () => {
    const connection: ProviderConnection = {
      provider: "Stooq",
      has_key: true,
      active_tier: "SessionMemory",
    };
    mockInvoke.mockResolvedValue(connection);

    const args: SaveProviderKeyArgs = {
      provider: "Stooq",
      key: "my-secret-key",
      allow_plaintext: false,
    };
    const result = await connectionGateway.saveProviderKey(args);

    expect(result).toEqual({ status: "ok", data: connection });
    if (result.status === "ok") expect(result.data.active_tier).toBe("SessionMemory");
  });

  // save_provider_key — allow_plaintext: true forwarded (KEY-012 tier-3 opt-in)
  it("saveProviderKey forwards allow_plaintext: true in args", async () => {
    const connection: ProviderConnection = {
      provider: "Stooq",
      has_key: true,
      active_tier: "PlaintextFile",
    };
    mockInvoke.mockResolvedValue(connection);

    const args: SaveProviderKeyArgs = {
      provider: "Stooq",
      key: "my-secret-key",
      allow_plaintext: true,
    };
    const result = await connectionGateway.saveProviderKey(args);

    expect(result).toEqual({ status: "ok", data: connection });
    expect(mockInvoke).toHaveBeenCalledWith("save_provider_key", { args });
  });

  // save_provider_key — EmptyKey error pass-through (KEY-010)
  it("saveProviderKey passes through EmptyKey error", async () => {
    const err: ConnectionError = { code: "EmptyKey" };
    mockInvoke.mockRejectedValue(err);

    const args: SaveProviderKeyArgs = {
      provider: "Stooq",
      key: "   ",
      allow_plaintext: false,
    };
    const result = await connectionGateway.saveProviderKey(args);

    expect(result).toEqual({ status: "error", error: err });
    if (result.status === "error") expect(result.error.code).toBe("EmptyKey");
  });

  // save_provider_key — KeyStoreError error pass-through (infrastructure failure)
  it("saveProviderKey passes through KeyStoreError error", async () => {
    const err: ConnectionError = { code: "KeyStoreError" };
    mockInvoke.mockRejectedValue(err);

    const args: SaveProviderKeyArgs = {
      provider: "Stooq",
      key: "some-key",
      allow_plaintext: false,
    };
    const result = await connectionGateway.saveProviderKey(args);

    expect(result).toEqual({ status: "error", error: err });
    if (result.status === "error") expect(result.error.code).toBe("KeyStoreError");
  });
});

describe("connection gateway — testProviderKey", () => {
  beforeEach(() => vi.clearAllMocks());

  // test_provider_key — Accepted outcome (KEY-023: outcomes are successful returns, not errors)
  it("testProviderKey passes through Accepted outcome", async () => {
    const outcome: ProviderKeyTestOutcome = "Accepted";
    mockInvoke.mockResolvedValue(outcome);

    const args: TestProviderKeyArgs = { provider: "Stooq", key: "valid-key" };
    const result = await connectionGateway.testProviderKey(args);

    expect(result).toEqual({ status: "ok", data: "Accepted" });
    expect(mockInvoke).toHaveBeenCalledWith("test_provider_key", { args });
  });

  // test_provider_key — Rejected outcome (KEY-023)
  it("testProviderKey passes through Rejected outcome", async () => {
    const outcome: ProviderKeyTestOutcome = "Rejected";
    mockInvoke.mockResolvedValue(outcome);

    const args: TestProviderKeyArgs = { provider: "Stooq", key: "wrong-key" };
    const result = await connectionGateway.testProviderKey(args);

    expect(result).toEqual({ status: "ok", data: "Rejected" });
    if (result.status === "ok") expect(result.data).toBe("Rejected");
  });

  // test_provider_key — Unreachable outcome (KEY-023)
  it("testProviderKey passes through Unreachable outcome", async () => {
    const outcome: ProviderKeyTestOutcome = "Unreachable";
    mockInvoke.mockResolvedValue(outcome);

    const args: TestProviderKeyArgs = { provider: "Stooq", key: "some-key" };
    const result = await connectionGateway.testProviderKey(args);

    expect(result).toEqual({ status: "ok", data: "Unreachable" });
    if (result.status === "ok") expect(result.data).toBe("Unreachable");
  });

  // test_provider_key — EmptyKey error (KEY-021: blank value rejected before probe)
  it("testProviderKey passes through EmptyKey error", async () => {
    const err: ConnectionError = { code: "EmptyKey" };
    mockInvoke.mockRejectedValue(err);

    const args: TestProviderKeyArgs = { provider: "Stooq", key: "" };
    const result = await connectionGateway.testProviderKey(args);

    expect(result).toEqual({ status: "error", error: err });
    if (result.status === "error") expect(result.error.code).toBe("EmptyKey");
  });

  // test_provider_key — args forwarded exactly (provider + key positional via args wrapper)
  it("testProviderKey forwards args object to the backend command", async () => {
    mockInvoke.mockResolvedValue("Accepted");

    const args: TestProviderKeyArgs = { provider: "Stooq", key: "test-key-123" };
    await connectionGateway.testProviderKey(args);

    expect(mockInvoke).toHaveBeenCalledWith("test_provider_key", { args });
  });
});

describe("connection gateway — removeProviderKey", () => {
  beforeEach(() => vi.clearAllMocks());

  // remove_provider_key — ok pass-through (returns null / unit, KEY-013)
  it("removeProviderKey passes through ok result with null data", async () => {
    mockInvoke.mockResolvedValue(null);

    const args: RemoveProviderKeyArgs = { provider: "Stooq" };
    const result = await connectionGateway.removeProviderKey(args);

    expect(result).toEqual({ status: "ok", data: null });
    expect(mockInvoke).toHaveBeenCalledWith("remove_provider_key", { args });
  });

  // remove_provider_key — KeyStoreError error pass-through
  it("removeProviderKey passes through KeyStoreError error", async () => {
    const err: ConnectionError = { code: "KeyStoreError" };
    mockInvoke.mockRejectedValue(err);

    const args: RemoveProviderKeyArgs = { provider: "Stooq" };
    const result = await connectionGateway.removeProviderKey(args);

    expect(result).toEqual({ status: "error", error: err });
    if (result.status === "error") expect(result.error.code).toBe("KeyStoreError");
  });

  // remove_provider_key — idempotent: removing when no key exists still succeeds (contract note)
  it("removeProviderKey passes through ok for idempotent removal", async () => {
    mockInvoke.mockResolvedValue(null);

    const args: RemoveProviderKeyArgs = { provider: "Stooq" };
    const result = await connectionGateway.removeProviderKey(args);

    expect(result.status).toBe("ok");
    expect(mockInvoke).toHaveBeenCalledWith("remove_provider_key", { args });
  });
});

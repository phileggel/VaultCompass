import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the accounts gateway before importing the hook under test.
// The hook calls accountGateway.fetchAllAssetPrices() — mock at this boundary
// so the hook never crosses into commands.* (F3).
vi.mock("../gateway", () => ({
  accountGateway: {
    fetchAllAssetPrices: vi.fn(),
  },
}));

// Mock the connections gateway — KEY-040 gate: hook reads provider connections to
// decide whether to dispatch or open the Connections dialog.
vi.mock("@/features/connections/gateway", () => ({
  connectionGateway: {
    getProviderConnections: vi.fn(),
  },
}));

// Mock router navigate — KEY-040: when no key, navigates to ?modal=connections
const mockNavigate = vi.hoisted(() => vi.fn());
vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

// Mock the snackbar store — the hook dispatches snackbar messages on all branches.
const mockShowSnackbar = vi.hoisted(() => vi.fn());
vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

import * as connectionGatewayModule from "@/features/connections/gateway";
import * as gateway from "../gateway";
import { useRefreshGlobalPrices } from "./useRefreshGlobalPrices";

describe("useRefreshGlobalPrices", () => {
  beforeEach(() => vi.clearAllMocks());

  // MKT-133 — isPending starts false
  it("isPending is false before refresh is called", () => {
    const { result } = renderHook(() => useRefreshGlobalPrices());
    expect(result.current.isPending).toBe(false);
  });

  // MKT-115 / MKT-133 — success path: snackbar mkt.fetch_dispatched, isPending returns to false
  it("dispatches mkt.fetch_dispatched snackbar on successful fetch dispatch", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_dispatched", expect.any(String));
    expect(result.current.isPending).toBe(false);
  });

  // MKT-115 — FetchAlreadyRunning → snackbar mkt.fetch_already_running
  it("dispatches mkt.fetch_already_running snackbar on FetchAlreadyRunning error", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "error",
      error: { code: "FetchAlreadyRunning" },
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_already_running", expect.any(String));
    expect(result.current.isPending).toBe(false);
  });

  // MKT-115 — NoFetchableHoldings → snackbar mkt.fetch_no_holdings
  it("dispatches mkt.fetch_no_holdings snackbar on NoFetchableHoldings error", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "error",
      error: { code: "NoFetchableHoldings" },
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_no_holdings", expect.any(String));
    expect(result.current.isPending).toBe(false);
  });

  // DatabaseError → snackbar error.DatabaseError
  it("dispatches error.DatabaseError snackbar on DatabaseError", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("error.DatabaseError", expect.any(String));
    expect(result.current.isPending).toBe(false);
  });

  // UnknownError → snackbar error.DatabaseError (generic fallback)
  it("dispatches error.DatabaseError snackbar on UnknownError", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "error",
      error: { code: "UnknownError" },
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("error.DatabaseError", expect.any(String));
    expect(result.current.isPending).toBe(false);
  });

  // MKT-133 — isPending is true while fetch is in flight
  it("isPending is true while the fetch gateway call is in progress", async () => {
    let resolveFetch!: (v: { status: "ok"; data: null }) => void;
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockReturnValue(
      new Promise((resolve) => {
        resolveFetch = resolve;
      }),
    );

    const { result } = renderHook(() => useRefreshGlobalPrices());

    act(() => {
      void result.current.refresh();
    });

    await waitFor(() => expect(result.current.isPending).toBe(true));

    await act(async () => {
      resolveFetch({ status: "ok", data: null });
    });

    expect(result.current.isPending).toBe(false);
  });

  // gateway is called exactly once per refresh() invocation
  it("calls accountGateway.fetchAllAssetPrices exactly once per refresh call", async () => {
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(gateway.accountGateway.fetchAllAssetPrices).toHaveBeenCalledTimes(1);
  });
});

describe("useRefreshGlobalPrices — KEY-040 key gate", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-040 — when Stooq has no key, navigate to ?modal=connections instead of dispatching
  it("navigates to ?modal=connections when Stooq has no key instead of dispatching fetch", async () => {
    vi.mocked(connectionGatewayModule.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(gateway.accountGateway.fetchAllAssetPrices).not.toHaveBeenCalled();
    expect(mockNavigate).toHaveBeenCalledWith(
      expect.objectContaining({ search: expect.objectContaining({ modal: "connections" }) }),
    );
  });

  // KEY-040 — when Stooq has a key, the fetch IS dispatched (gate passes)
  it("dispatches fetch when Stooq has a key stored", async () => {
    vi.mocked(connectionGatewayModule.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(gateway.accountGateway.fetchAllAssetPrices).toHaveBeenCalledTimes(1);
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  // KEY-040 — gate check uses getProviderConnections, not a static import
  it("calls connectionGateway.getProviderConnections on each refresh to check key status", async () => {
    vi.mocked(connectionGatewayModule.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });
    vi.mocked(gateway.accountGateway.fetchAllAssetPrices).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useRefreshGlobalPrices());

    await act(async () => {
      await result.current.refresh();
    });

    expect(connectionGatewayModule.connectionGateway.getProviderConnections).toHaveBeenCalledTimes(
      1,
    );
  });
});

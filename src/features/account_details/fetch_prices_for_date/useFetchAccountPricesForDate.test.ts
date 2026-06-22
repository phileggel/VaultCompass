import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the account_details gateway at the boundary — the hook never crosses into
// commands.* directly (F3).
vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    fetchAccountAssetPricesForDate: vi.fn(),
  },
}));

// Mock the snackbar store — the hook dispatches snackbar messages on every branch.
const mockShowSnackbar = vi.hoisted(() => vi.fn());
vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

import * as gateway from "../gateway";
import { useFetchAccountPricesForDate } from "./useFetchAccountPricesForDate";

const mockedFetch = vi.mocked(gateway.accountDetailsGateway.fetchAccountAssetPricesForDate);

describe("useFetchAccountPricesForDate", () => {
  beforeEach(() => vi.clearAllMocks());

  it("defaults the date to today (ISO yyyy-mm-dd) and is not submitting", () => {
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", vi.fn()));
    expect(result.current.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(result.current.isSubmitting).toBe(false);
  });

  it("setDate updates the date passed to the gateway on submit", async () => {
    mockedFetch.mockResolvedValue({ status: "ok", data: { stored: 1, missing: [] } });
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-7", vi.fn()));

    act(() => result.current.setDate("2024-06-10"));
    await act(async () => {
      await result.current.submit();
    });

    expect(mockedFetch).toHaveBeenCalledWith("account-7", "2024-06-10");
  });

  it("dispatches success snackbar and calls onDone when all assets stored", async () => {
    mockedFetch.mockResolvedValue({ status: "ok", data: { stored: 3, missing: [] } });
    const onDone = vi.fn();
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", onDone));

    await act(async () => {
      await result.current.submit();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_date_stored", "success");
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(result.current.isSubmitting).toBe(false);
  });

  it("dispatches info snackbar and calls onDone when some assets are missing", async () => {
    mockedFetch.mockResolvedValue({ status: "ok", data: { stored: 2, missing: ["Acme"] } });
    const onDone = vi.fn();
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", onDone));

    await act(async () => {
      await result.current.submit();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_date_partial", "info");
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("dispatches the date-future error snackbar and does NOT call onDone on DateInFuture", async () => {
    mockedFetch.mockResolvedValue({ status: "error", error: { code: "DateInFuture" } });
    const onDone = vi.fn();
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", onDone));

    await act(async () => {
      await result.current.submit();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.fetch_date_future", "error");
    expect(onDone).not.toHaveBeenCalled();
  });

  it("dispatches the AccountNotFound error snackbar on AccountNotFound", async () => {
    mockedFetch.mockResolvedValue({
      status: "error",
      error: { code: "AccountNotFound", account_id: "account-1" },
    });
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", vi.fn()));

    await act(async () => {
      await result.current.submit();
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("error.AccountNotFound", "error");
  });

  it("isSubmitting is true while the fetch is in flight", async () => {
    let resolveFetch!: (v: { status: "ok"; data: { stored: number; missing: string[] } }) => void;
    mockedFetch.mockReturnValue(
      new Promise((resolve) => {
        resolveFetch = resolve;
      }),
    );
    const { result } = renderHook(() => useFetchAccountPricesForDate("account-1", vi.fn()));

    act(() => {
      void result.current.submit();
    });
    await waitFor(() => expect(result.current.isSubmitting).toBe(true));

    await act(async () => {
      resolveFetch({ status: "ok", data: { stored: 1, missing: [] } });
    });
    expect(result.current.isSubmitting).toBe(false);
  });
});

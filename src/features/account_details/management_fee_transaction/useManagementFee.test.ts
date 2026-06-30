import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { logger } from "@/lib/logger";
import { useManagementFee } from "./useManagementFee";

// ── Hoisted mocks ──────────────────────────────────────────────────────────────
const {
  mockRecordManagementFee,
  mockShowSnackbar,
  mockGetLastOperationDate,
  mockSetLastOperationDate,
} = vi.hoisted(() => ({
  mockRecordManagementFee: vi.fn(),
  mockShowSnackbar: vi.fn(),
  mockGetLastOperationDate: vi.fn(),
  mockSetLastOperationDate: vi.fn(),
}));

vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    recordManagementFee: mockRecordManagementFee,
  },
}));

vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

vi.mock("@/lib/logger", () => ({
  logger: { error: vi.fn(), info: vi.fn(), warn: vi.fn() },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

vi.mock("@/lib/lastOperationDateStorage", () => ({
  getLastOperationDate: mockGetLastOperationDate,
  setLastOperationDate: mockSetLastOperationDate,
}));

// ── Fixtures ───────────────────────────────────────────────────────────────────
const TODAY = new Date().toISOString().slice(0, 10);

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

const BASE_PROPS = {
  accountId: "account-1",
  onSubmitSuccess: vi.fn(),
};

// ── Tests ──────────────────────────────────────────────────────────────────────
describe("useManagementFee (FEE-020/021/022)", () => {
  beforeEach(() => {
    mockRecordManagementFee.mockReset();
    mockShowSnackbar.mockReset();
    mockGetLastOperationDate.mockReturnValue(TODAY);
    mockSetLastOperationDate.mockReset();
    vi.mocked(logger.error).mockClear();
    BASE_PROPS.onSubmitSuccess.mockClear();
  });

  // FEE-021 — initial form has the last-operation date, no asset, blank percent, blank note
  it("initial state has the last-operation date, no asset selected, blank percent, and blank note", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    expect(result.current.formData.date).toBe(TODAY);
    expect(result.current.formData.assetId).toBe("");
    expect(result.current.formData.percent).toBe("");
    expect(result.current.formData.note).toBe("");
  });

  // FEE-021 — form invalid when no asset selected
  it("isFormValid false when no asset is selected", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => result.current.handleChange("percent", "1.5"));
    expect(result.current.isFormValid).toBe(false);
  });

  // FEE-021 — form invalid when percent is blank
  it("isFormValid false when asset selected but percent is blank", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => result.current.handleChange("assetId", "asset-equity-1"));
    expect(result.current.isFormValid).toBe(false);
  });

  // FEE-021 — form invalid when percent is zero
  it("isFormValid false when percent is zero", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "0");
    });
    expect(result.current.isFormValid).toBe(false);
  });

  // FEE-021 — form invalid when percent is negative
  it("isFormValid false when percent is negative", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "-1");
    });
    expect(result.current.isFormValid).toBe(false);
  });

  // FEE-021 — form invalid when percent exceeds 100
  it("isFormValid false when percent exceeds 100", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "100.01");
    });
    expect(result.current.isFormValid).toBe(false);
  });

  // FEE-021 — form valid with asset + positive percent in range + valid date
  it("isFormValid true when asset selected, percent in (0, 100], and date valid", () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1.5");
    });
    expect(result.current.isFormValid).toBe(true);
  });

  // FEE-022 — valid submit calls gateway with percent_micros and null note when blank
  it("submits and calls gateway with percent_micros and null note when note is empty", async () => {
    mockRecordManagementFee.mockResolvedValue({ status: "ok", data: { id: "tx-fee-1" } });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1.5");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordManagementFee).toHaveBeenCalledWith(
      expect.objectContaining({
        account_id: "account-1",
        asset_id: "asset-equity-1",
        percent_micros: 1_500_000,
        note: null,
      }),
    );
  });

  // FEE-022 — note is passed through when non-empty
  it("passes a non-empty note to the gateway", async () => {
    mockRecordManagementFee.mockResolvedValue({ status: "ok", data: { id: "tx-fee-1" } });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "2");
      result.current.handleChange("note", "Q2 fee");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordManagementFee).toHaveBeenCalledWith(
      expect.objectContaining({ note: "Q2 fee" }),
    );
  });

  // FEE-025 — success: snackbar shown, date stored, onSubmitSuccess called
  it("shows success snackbar, stores last-operation date, and calls onSubmitSuccess on ok result", async () => {
    mockRecordManagementFee.mockResolvedValue({ status: "ok", data: { id: "tx-fee-1" } });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("management_fee.recorded", "success");
    expect(mockSetLastOperationDate).toHaveBeenCalledWith("account-1", TODAY);
    expect(BASE_PROPS.onSubmitSuccess).toHaveBeenCalled();
  });

  // FEE-025 — error result sets inline error (F27 no-throw)
  it("surfaces backend error code as inline error on error result (F27)", async () => {
    mockRecordManagementFee.mockResolvedValue({
      status: "error",
      error: { code: "AssetNotHeld" },
    });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toEqual({ key: "error.AssetNotHeld" });
    expect(mockShowSnackbar).not.toHaveBeenCalled();
  });

  // FEE-011 — ManagementFeeOnCashAsset maps to its i18n key
  it("maps ManagementFeeOnCashAsset error to its i18n key", async () => {
    mockRecordManagementFee.mockResolvedValue({
      status: "error",
      error: { code: "ManagementFeeOnCashAsset" },
    });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toEqual({ key: "error.ManagementFeeOnCashAsset" });
  });

  // DatabaseError — logged and mapped to i18n key
  it("logs and maps DatabaseError to inline i18n key", async () => {
    mockRecordManagementFee.mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toEqual({ key: "error.DatabaseError" });
    expect(logger.error).toHaveBeenCalledWith(
      "[useManagementFee] recordManagementFee failed",
      expect.objectContaining({ error: { code: "DatabaseError" } }),
    );
    expect(mockShowSnackbar).not.toHaveBeenCalled();
  });

  // FEE-025 — isSubmitting toggles during the request
  it("isSubmitting is true during submit and false after", async () => {
    let resolvePromise!: (value: unknown) => void;
    mockRecordManagementFee.mockReturnValue(
      new Promise((resolve) => {
        resolvePromise = resolve;
      }),
    );
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));

    act(() => {
      result.current.handleChange("assetId", "asset-equity-1");
      result.current.handleChange("percent", "1");
    });

    act(() => {
      void result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.isSubmitting).toBe(true);

    await act(async () => {
      resolvePromise({ status: "ok", data: { id: "tx-fee-1" } });
    });

    expect(result.current.isSubmitting).toBe(false);
  });

  // FEE-021 — submit blocked when form is invalid (no gateway call)
  it("handleSubmit does not call gateway when form is invalid", async () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    // No assetId, no percent — form is invalid

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordManagementFee).not.toHaveBeenCalled();
  });

  // FEE-021 — validation error set when assetId missing at submit time
  it("sets error.AssetNotHeld when submit attempted with no asset selected", async () => {
    const { result } = renderHook(() => useManagementFee(BASE_PROPS));
    act(() => result.current.handleChange("percent", "1.5"));

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toEqual({ key: "error.AssetNotHeld" });
    expect(mockRecordManagementFee).not.toHaveBeenCalled();
  });
});

import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Asset } from "@/bindings";
import { SYSTEM_CATEGORY_ID } from "../shared/constants";
import { useAddAsset } from "./useAddAsset";

const mockAddAsset = vi.fn();

vi.mock("../useAssets", () => ({
  useAssets: () => ({
    addAsset: mockAddAsset,
    assets: [] as Asset[],
    activeCount: 0,
    loading: false,
    fetchError: null,
    fetchAssets: vi.fn(),
    updateAsset: vi.fn(),
    archiveAsset: vi.fn(),
    unarchiveAsset: vi.fn(),
    deleteAsset: vi.fn(),
  }),
}));

vi.mock("@/features/categories/useCategories", () => ({
  useCategories: () => ({
    categories: [{ id: "cat-1", name: "Bonds", is_system: false }],
    loading: false,
  }),
}));

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

describe("useAddAsset", () => {
  beforeEach(() => {
    mockAddAsset.mockReset();
  });

  // R2 — category pre-selected on SYSTEM_CATEGORY_ID
  it("pre-selects SYSTEM_CATEGORY_ID as default category", () => {
    const { result } = renderHook(() => useAddAsset());
    expect(result.current.formData.category_id).toBe(SYSTEM_CATEGORY_ID);
  });

  // R10 — risk_level auto-filled when class changes
  it("auto-fills risk_level when class changes", () => {
    const { result } = renderHook(() => useAddAsset());

    act(() => {
      result.current.handleClassChange("DigitalAsset");
    });

    expect(result.current.formData.class).toBe("DigitalAsset");
    expect(result.current.formData.risk_level).toBe(5);
  });

  // R10 — risk default for Stocks is 4
  it("sets risk_level to 4 when class is Stocks", () => {
    const { result } = renderHook(() => useAddAsset());

    act(() => {
      result.current.handleClassChange("Stocks");
    });

    expect(result.current.formData.risk_level).toBe(4);
  });

  // R9 — hasDuplicateReference is tested exhaustively in validateAsset.test.ts
  // Here we verify the hook integrates it without crashing (assets mock is [] in this scope)
  it("exposes duplicateWarning computed from assets", () => {
    const { result } = renderHook(() => useAddAsset());

    act(() => {
      result.current.handleChange({
        target: { name: "reference", value: "AAPL" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    expect(typeof result.current.duplicateWarning).toBe("boolean");
  });

  // R14 — modal stays open on backend error, exposes error message
  it("does not call onSubmitSuccess and exposes error on backend failure", async () => {
    mockAddAsset.mockResolvedValue({ data: null, error: "Backend error" });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAsset({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { name: "name", value: "Apple" },
      } as React.ChangeEvent<HTMLInputElement>);
      result.current.handleChange({
        target: { name: "reference", value: "AAPL" },
      } as React.ChangeEvent<HTMLInputElement>);
      result.current.handleChange({
        target: { name: "currency", value: "USD" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("Backend error");
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // R14 — success closes modal
  it("calls onSubmitSuccess and clears error on success", async () => {
    mockAddAsset.mockResolvedValue({ data: { id: "new" }, error: null });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAsset({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { name: "name", value: "Apple" },
      } as React.ChangeEvent<HTMLInputElement>);
      result.current.handleChange({
        target: { name: "reference", value: "AAPL" },
      } as React.ChangeEvent<HTMLInputElement>);
      result.current.handleChange({
        target: { name: "currency", value: "USD" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBeNull();
    expect(onSubmitSuccess).toHaveBeenCalledTimes(1);
  });
});

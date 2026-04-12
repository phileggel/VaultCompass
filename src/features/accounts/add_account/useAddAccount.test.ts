import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddAccount } from "./useAddAccount";

const mockAddAccount = vi.fn();

vi.mock("../useAccounts", () => ({
  useAccounts: () => ({
    addAccount: mockAddAccount,
    accounts: [],
    loading: false,
    fetchError: null,
    fetchAccounts: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
  }),
}));

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

describe("useAddAccount", () => {
  beforeEach(() => {
    mockAddAccount.mockReset();
  });

  // R14 — empty name blocks submission
  it("sets error and does not call addAccount when name is empty", async () => {
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAccount({ onSubmitSuccess }));

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).not.toBeNull();
    expect(mockAddAccount).not.toHaveBeenCalled();
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // R14 — whitespace-only name blocks submission
  it("sets error and does not call addAccount when name is whitespace only", async () => {
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAccount({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { name: "name", value: "   " },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).not.toBeNull();
    expect(mockAddAccount).not.toHaveBeenCalled();
  });

  // R13 — backend error keeps modal open and exposes error
  it("sets error and does not call onSubmitSuccess on backend error", async () => {
    mockAddAccount.mockResolvedValue({ data: null, error: "Duplicate name" });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAccount({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { name: "name", value: "Alpha" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("Duplicate name");
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // R13 — success calls onSubmitSuccess and clears error
  it("calls onSubmitSuccess and clears error on success", async () => {
    mockAddAccount.mockResolvedValue({ data: { id: "new-id" }, error: null });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddAccount({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { name: "name", value: "Alpha" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBeNull();
    expect(onSubmitSuccess).toHaveBeenCalledTimes(1);
  });
});

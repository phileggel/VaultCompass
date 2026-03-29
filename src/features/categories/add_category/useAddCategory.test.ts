import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddCategory } from "./useAddCategory";

const mockAddCategory = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("../useCategories", () => ({
  useCategories: () => ({
    addCategory: mockAddCategory,
    updateCategory: vi.fn(),
    deleteCategory: vi.fn(),
    categories: [],
    loading: false,
    error: null,
    fetchCategories: vi.fn(),
  }),
}));

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

describe("useAddCategory", () => {
  beforeEach(() => {
    mockAddCategory.mockReset();
  });

  it("calls addCategory and invokes onSubmitSuccess on success", async () => {
    mockAddCategory.mockResolvedValue(undefined);
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddCategory({ onSubmitSuccess }));

    act(() => {
      result.current.handleChange({
        target: { value: "Bonds" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockAddCategory).toHaveBeenCalledWith("Bonds");
    expect(onSubmitSuccess).toHaveBeenCalledTimes(1);
    expect(result.current.name).toBe("");
    expect(result.current.error).toBeNull();
  });

  it("does not call addCategory when name is empty", async () => {
    const { result } = renderHook(() => useAddCategory());

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockAddCategory).not.toHaveBeenCalled();
  });

  it("sets error_duplicate when addCategory rejects with duplicate_name", async () => {
    mockAddCategory.mockRejectedValue(new Error("error.category.duplicate_name"));
    const { result } = renderHook(() => useAddCategory());

    act(() => {
      result.current.handleChange({
        target: { value: "Bonds" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("category.error_duplicate");
  });

  it("sets error_generic for unknown errors", async () => {
    mockAddCategory.mockRejectedValue(new Error("something went wrong"));
    const { result } = renderHook(() => useAddCategory());

    act(() => {
      result.current.handleChange({
        target: { value: "Bonds" },
      } as React.ChangeEvent<HTMLInputElement>);
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("category.error_generic");
  });
});

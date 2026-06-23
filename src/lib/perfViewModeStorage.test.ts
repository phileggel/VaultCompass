import { afterEach, describe, expect, it } from "vitest";
import { getPerfViewMode, setPerfViewMode } from "./perfViewModeStorage";

describe("perfViewModeStorage", () => {
  afterEach(() => localStorage.clear());

  it("returns null when no preference is stored for the account", () => {
    expect(getPerfViewMode("acc-1")).toBeNull();
  });

  it("returns null for an empty account id", () => {
    expect(getPerfViewMode("")).toBeNull();
  });

  it("round-trips a stored view mode per account", () => {
    setPerfViewMode("acc-1", "month");
    setPerfViewMode("acc-2", "year");
    expect(getPerfViewMode("acc-1")).toBe("month");
    expect(getPerfViewMode("acc-2")).toBe("year");
  });

  it("overwrites a stored mode (last write wins)", () => {
    setPerfViewMode("acc-1", "month");
    setPerfViewMode("acc-1", "year");
    expect(getPerfViewMode("acc-1")).toBe("year");
  });

  it("ignores a write with an empty account id", () => {
    setPerfViewMode("", "month");
    expect(getPerfViewMode("")).toBeNull();
  });

  it("treats an unrecognized stored value as no preference", () => {
    localStorage.setItem("perf_view_mode_acc-1", "weekly");
    expect(getPerfViewMode("acc-1")).toBeNull();
  });
});

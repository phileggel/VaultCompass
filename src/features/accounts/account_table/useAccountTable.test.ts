import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Account } from "@/bindings";
import { useAccountTable } from "./useAccountTable";

function makeAccount(id: string, name: string, freq: Account["update_frequency"]): Account {
  return { id, name, currency: "EUR", update_frequency: freq };
}

const accounts: Account[] = [
  makeAccount("1", "Alpha", "ManualYear"),
  makeAccount("2", "Beta", "Automatic"),
  makeAccount("3", "Gamma", "ManualDay"),
  makeAccount("4", "Delta", "ManualMonth"),
];

const noopDelete = vi.fn().mockResolvedValue({ error: null });

describe("useAccountTable", () => {
  // R9 — frequency sorted by logical enum order, not alphabetical label
  it("sorts update_frequency by logical enum order ascending", () => {
    const { result } = renderHook(() => useAccountTable(accounts, "", noopDelete));

    act(() => {
      result.current.handleSort("update_frequency");
    });

    const freqs = result.current.sortedAndFilteredAccounts.map((a) => a.update_frequency);
    expect(freqs).toEqual(["Automatic", "ManualDay", "ManualMonth", "ManualYear"]);
  });

  // R9 — descending reverses logical order
  it("sorts update_frequency by logical enum order descending on second click", () => {
    const { result } = renderHook(() => useAccountTable(accounts, "", noopDelete));

    act(() => result.current.handleSort("update_frequency"));
    act(() => result.current.handleSort("update_frequency"));

    const freqs = result.current.sortedAndFilteredAccounts.map((a) => a.update_frequency);
    expect(freqs).toEqual(["ManualYear", "ManualMonth", "ManualDay", "Automatic"]);
  });

  // R10 — search active with no match → hasNoSearchResults true
  it("sets hasNoSearchResults when filter is active but no match", () => {
    const { result } = renderHook(() => useAccountTable(accounts, "zzz", noopDelete));
    expect(result.current.hasNoSearchResults).toBe(true);
    expect(result.current.isEmpty).toBe(false);
  });

  // R11 — empty list with no filter → isEmpty true
  it("sets isEmpty when list is empty and no search is active", () => {
    const { result } = renderHook(() => useAccountTable([], "", noopDelete));
    expect(result.current.isEmpty).toBe(true);
    expect(result.current.hasNoSearchResults).toBe(false);
  });

  // R10 / R11 — empty list with active filter → hasNoSearchResults (not isEmpty)
  it("sets hasNoSearchResults (not isEmpty) when list is empty but filter is active", () => {
    const { result } = renderHook(() => useAccountTable([], "something", noopDelete));
    expect(result.current.hasNoSearchResults).toBe(true);
    expect(result.current.isEmpty).toBe(false);
  });
});

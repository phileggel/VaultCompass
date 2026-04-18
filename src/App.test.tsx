import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import App from "./App";

// Mock Tauri event system (used by useUpdateBanner and db:migration_error listener)
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// Mock Tauri bindings
vi.mock("./bindings", () => ({
  commands: {
    getAssets: vi.fn(() => Promise.resolve({ status: "ok", data: [] })),
    getAssetsWithArchived: vi.fn(() => Promise.resolve({ status: "ok", data: [] })),
    addAsset: vi.fn(),
    deleteAsset: vi.fn(),
    getAccountTypes: vi.fn(() => Promise.resolve({ status: "ok", data: [] })),
    addAccountType: vi.fn(),
    deleteAccountType: vi.fn(),
    getAccounts: vi.fn(() => Promise.resolve({ status: "ok", data: [] })),
    getCategories: vi.fn(() => Promise.resolve({ status: "ok", data: [] })),
    checkForUpdate: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
    downloadUpdate: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
    installUpdate: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
  },
  events: {
    event: {
      listen: vi.fn(() => Promise.resolve(() => {})),
    },
  },
}));

test("renders VaultCompass title", async () => {
  render(<App />);
  const titleElements = await screen.findAllByText(/VaultCompass/i);
  expect(titleElements.length).toBeGreaterThan(0);
});

test("renders Assets navigation item", async () => {
  render(<App />);
  const assetsElements = await screen.findAllByText(/Assets/i);
  expect(assetsElements.length).toBeGreaterThan(0);
});

test("renders Categories navigation item", async () => {
  render(<App />);
  const categoriesElements = await screen.findAllByText(/Categories/i);
  expect(categoriesElements.length).toBeGreaterThan(0);
});

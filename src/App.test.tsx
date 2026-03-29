import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import App from "./App";

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
  },
  events: {
    event: {
      listen: vi.fn(() => Promise.resolve(() => {})),
    },
  },
}));

test("renders Vault M3 title", async () => {
  render(<App />);
  const titleElements = await screen.findAllByText(/Vault M3/i);
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

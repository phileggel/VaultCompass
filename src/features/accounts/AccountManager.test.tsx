import { fireEvent, render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AccountManager } from "./AccountManager";

const { mockNavigate } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

vi.mock("@/lib/logger", () => ({ logger: { info: vi.fn(), error: vi.fn() } }));

// Stub the children so the manager renders in isolation (no gateway, no Tauri).
vi.mock("./account_table/AccountTable", () => ({
  AccountTable: () => <div data-testid="account-table" />,
}));
vi.mock("./add_account/AddAccountModal", () => ({ AddAccountModal: () => null }));
vi.mock("./refresh_prices/useRefreshGlobalPrices", () => ({
  useRefreshGlobalPrices: () => ({ isPending: false, refresh: vi.fn() }),
}));

describe("AccountManager — global performance entry point (GPF)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the performance entry button next to the search field", () => {
    render(<AccountManager />);
    expect(document.querySelector("#accounts-performance")).toBeInTheDocument();
  });

  it("navigates to /performance when the entry button is clicked", () => {
    render(<AccountManager />);
    fireEvent.click(document.querySelector("#accounts-performance")!);
    expect(mockNavigate).toHaveBeenCalledWith({ to: "/performance" });
  });
});

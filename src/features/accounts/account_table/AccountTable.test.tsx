import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AccountSummary } from "@/bindings";
import { AccountTable } from "./AccountTable";

// Strip i18n — keys come through unchanged for stable assertions.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) =>
      vars ? `${key} ${JSON.stringify(vars)}` : key,
    i18n: { language: "en" },
  }),
}));

const mockSummaries = vi.fn<() => AccountSummary[]>();
const mockIsLoading = vi.fn<() => boolean>(() => false);
const mockError = vi.fn<() => unknown>(() => null);
const mockRefetch = vi.fn();

vi.mock("../useAccountSummaries", () => ({
  useAccountSummaries: () => ({
    summaries: mockSummaries(),
    isLoading: mockIsLoading(),
    error: mockError(),
    refetch: mockRefetch,
  }),
}));

vi.mock("../useAccounts", () => ({
  useAccounts: () => ({
    deleteAccount: vi.fn(),
    getAccountDeletionSummary: vi.fn(),
  }),
}));

// Stub out the child modals; we're testing the table chrome, not the dialogs.
vi.mock("../edit_account_modal/EditAccountModal", () => ({
  EditAccountModal: () => null,
}));
vi.mock("@/ui/components/modal/Dialog", async () => {
  const actual = (await vi.importActual("@/ui/components/modal/Dialog")) as Record<string, unknown>;
  return { ...actual, ConfirmationDialog: () => null };
});

// Force English number formatting so the assertion matches across locales.
vi.mock("@/lib/microUnits", () => ({
  microToFormatted: (micros: number, decimals = 3) =>
    (micros / 1_000_000).toLocaleString("en-US", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }),
}));

const makeSummary = (overrides: Partial<AccountSummary> = {}): AccountSummary => ({
  id: "acc-1",
  name: "Main EUR",
  currency: "EUR",
  update_frequency: "ManualMonth",
  total_global_value: 100_000_000,
  ...overrides,
});

describe("AccountTable — Global Value column (ACC-021)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsLoading.mockReturnValue(false);
    mockError.mockReturnValue(null);
  });

  it("renders the Global Value column header in the table", () => {
    mockSummaries.mockReturnValue([makeSummary()]);
    render(<AccountTable searchTerm="" onAccountClick={vi.fn()} />);
    expect(screen.getByText("account.column_global_value")).toBeInTheDocument();
  });

  it("formats each row's value with currency suffix", () => {
    mockSummaries.mockReturnValue([
      makeSummary({
        id: "a",
        name: "EUR Account",
        currency: "EUR",
        total_global_value: 47_250_000_000,
      }),
      makeSummary({
        id: "b",
        name: "USD Account",
        currency: "USD",
        total_global_value: 132_000_000_000,
      }),
    ]);
    render(<AccountTable searchTerm="" onAccountClick={vi.fn()} />);

    expect(screen.getByText("47,250.00")).toBeInTheDocument();
    expect(screen.getByText("132,000.00")).toBeInTheDocument();
    // Per-row currency suffix renders next to each value.
    expect(screen.getAllByText("EUR")).not.toHaveLength(0);
    expect(screen.getAllByText("USD")).not.toHaveLength(0);
  });

  it("sorts rows by total_global_value when the header is clicked, toggling asc/desc", () => {
    mockSummaries.mockReturnValue([
      makeSummary({ id: "a", name: "Big", total_global_value: 9_000_000_000 }),
      makeSummary({ id: "b", name: "Small", total_global_value: 1_000_000 }),
    ]);
    render(<AccountTable searchTerm="" onAccountClick={vi.fn()} />);

    // Default sort is by name asc → "Big" first.
    const firstBodyRow = () => screen.getAllByRole("row")[1] as HTMLElement;
    const valueHeader = () =>
      screen.getByText("account.column_global_value").closest("th") as HTMLElement;
    expect(firstBodyRow().textContent).toContain("Big");
    expect(valueHeader().getAttribute("aria-sort")).toBe("none");

    // First click → ascending by value → "Small" first; aria-sort="ascending".
    fireEvent.click(screen.getByText("account.column_global_value"));
    expect(firstBodyRow().textContent).toContain("Small");
    expect(valueHeader().getAttribute("aria-sort")).toBe("ascending");

    // Second click flips to descending → "Big" first again; aria-sort="descending".
    fireEvent.click(screen.getByText("account.column_global_value"));
    expect(firstBodyRow().textContent).toContain("Big");
    expect(valueHeader().getAttribute("aria-sort")).toBe("descending");
  });

  it("renders 0 as the value when an account has no holdings", () => {
    mockSummaries.mockReturnValue([
      makeSummary({ id: "empty", name: "Empty", currency: "JPY", total_global_value: 0 }),
    ]);
    render(<AccountTable searchTerm="" onAccountClick={vi.fn()} />);
    // The value cell should contain "0.00" formatted and the JPY currency suffix.
    const row = screen.getAllByRole("row")[1] as HTMLElement;
    expect(within(row).getByText("0.00")).toBeInTheDocument();
    expect(within(row).getByText("JPY")).toBeInTheDocument();
  });
});

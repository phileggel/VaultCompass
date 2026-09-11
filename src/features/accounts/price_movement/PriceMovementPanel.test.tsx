import { configure, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PriceMovementReport, PriceMovementRow } from "@/bindings";
import { PriceMovementPanel } from "./PriceMovementPanel";

// F25 — stable ids are the selector surface; resolve getByTestId against `id`.
configure({ testIdAttribute: "id" });

// Strip i18n — keys come through unchanged (vars appended as JSON) for stable
// assertions (F16/F24). No `useTranslation` in the presenter layer (F27), only here.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) =>
      vars ? `${key} ${JSON.stringify(vars)}` : key,
    i18n: { language: "en" },
  }),
}));

// Default row/report fixtures describe ONE moved account, so most tests land
// in the "moved" (table) state without repeating the override every time.
const makeRow = (overrides: Partial<PriceMovementRow> = {}): PriceMovementRow => ({
  account_id: "acc-1",
  name: "Main",
  currency: "EUR",
  before: 100_000_000,
  after: 120_000_000,
  movement_pct: 20_000_000,
  incomplete: false,
  ...overrides,
});

const makeReport = (overrides: Partial<PriceMovementReport> = {}): PriceMovementReport => ({
  rows: [makeRow()],
  total_before: 100_000_000,
  total_after: 120_000_000,
  total_currency: "EUR",
  total_movement_pct: 20_000_000,
  observed_from: "2026-09-09",
  observed_to: "2026-09-11",
  incomplete: false,
  ...overrides,
});

describe("PriceMovementPanel", () => {
  beforeEach(() => vi.clearAllMocks());

  // PMV-013 — a dismissible panel, never a modal: no dialog role, no scrim.
  it("renders as a plain panel, not a modal", () => {
    render(<PriceMovementPanel report={makeReport()} onDismiss={vi.fn()} />);

    expect(screen.getByTestId("price-movement-panel")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  // PMV-017 — figures are framed as a dated before/after of THIS refresh, not
  // the account's current value shown a few rows below.
  it("states the figures are a frozen comparison of this refresh", () => {
    render(<PriceMovementPanel report={makeReport()} onDismiss={vi.fn()} />);

    expect(screen.getByText("pmv.subtitle_frozen")).toBeInTheDocument();
  });

  // PMV-061 — dismiss calls the caller-supplied handler; the panel owns no
  // persistence of its own.
  it("calls onDismiss when the dismiss control is clicked", () => {
    const onDismiss = vi.fn();
    render(<PriceMovementPanel report={makeReport()} onDismiss={onDismiss} />);

    fireEvent.click(screen.getByTestId("price-movement-dismiss"));

    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  // PMV-060 — nothing moved: a plain statement, no table.
  it("shows a plain 'nothing moved' statement instead of a table when every account is unmoved", () => {
    const report = makeReport({
      rows: [
        makeRow({
          account_id: "acc-1",
          before: 100_000_000,
          after: 100_000_000,
          movement_pct: null,
        }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.getByText("pmv.nothing_moved")).toBeInTheDocument();
    expect(screen.queryByTestId("price-movement-row-acc-1")).not.toBeInTheDocument();
  });

  // PMV-060/PMV-032 — nothing moved, but also incomplete: state both, never
  // mistaking "nothing moved" for "nothing was tried".
  it("states incompleteness together with 'nothing moved' when the report is incomplete", () => {
    const report = makeReport({
      rows: [
        makeRow({
          account_id: "acc-1",
          before: 100_000_000,
          after: 100_000_000,
          movement_pct: null,
          incomplete: true,
        }),
      ],
      incomplete: true,
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.getByText("pmv.nothing_moved_incomplete")).toBeInTheDocument();
    expect(screen.queryByText("pmv.nothing_moved")).not.toBeInTheDocument();
  });

  // PMV-030 — every account is listed, including one that did not move.
  it("renders one row per account, including an unmoved one", () => {
    const report = makeReport({
      rows: [
        makeRow({ account_id: "acc-1", name: "Main", before: 100_000_000, after: 120_000_000 }),
        makeRow({
          account_id: "acc-2",
          name: "Savings",
          before: 50_000_000,
          after: 50_000_000,
          movement_pct: null,
        }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.getByTestId("price-movement-row-acc-1")).toBeInTheDocument();
    expect(screen.getByTestId("price-movement-row-acc-2")).toBeInTheDocument();
  });

  // PMV-031 — an unmoved account renders no movement figure, never "0.00%".
  it("renders '—' for an unmoved account instead of a zero percentage", () => {
    const report = makeReport({
      rows: [
        makeRow({ account_id: "acc-1", before: 100_000_000, after: 120_000_000 }),
        makeRow({
          account_id: "acc-2",
          before: 50_000_000,
          after: 50_000_000,
          movement_pct: null,
        }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const row = within(screen.getByTestId("price-movement-row-acc-2"));
    expect(row.getByText("—")).toBeInTheDocument();
    expect(row.queryByText("0,00%")).not.toBeInTheDocument();
  });

  // PMV-024 — a moved account carries its computed percentage.
  it("renders the movement percentage for a moved account", () => {
    const report = makeReport({
      rows: [
        makeRow({
          account_id: "acc-1",
          before: 100_000_000,
          after: 120_000_000,
          movement_pct: 20_000_000,
        }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const row = within(screen.getByTestId("price-movement-row-acc-1"));
    expect(row.getByText("+20,00%")).toBeInTheDocument();
  });

  // PMV-032/043 — an incomplete row and the incomplete total are both marked.
  it("marks an incomplete row and the incomplete total", () => {
    const report = makeReport({
      rows: [makeRow({ account_id: "acc-1", incomplete: true })],
      incomplete: true,
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const row = within(screen.getByTestId("price-movement-row-acc-1"));
    expect(row.getByText("pmv.incomplete_marker")).toBeInTheDocument();
    const total = within(screen.getByTestId("price-movement-total"));
    expect(total.getByText("pmv.incomplete_marker")).toBeInTheDocument();
  });

  // PMV-043 — a complete report marks neither the rows nor the total.
  it("does not mark a complete row or the complete total", () => {
    render(<PriceMovementPanel report={makeReport()} onDismiss={vi.fn()} />);

    const row = within(screen.getByTestId("price-movement-row-acc-1"));
    expect(row.queryByText("pmv.incomplete_marker")).not.toBeInTheDocument();
    const total = within(screen.getByTestId("price-movement-total"));
    expect(total.queryByText("pmv.incomplete_marker")).not.toBeInTheDocument();
  });

  // PMV-033 — rows render in the backend's own order (alphabetical by name,
  // per PMV-033), never re-sorted by the frontend.
  it("renders rows in the order the report supplies, not re-sorted", () => {
    const report = makeReport({
      rows: [
        makeRow({
          account_id: "acc-z",
          name: "Zebra Account",
          before: 100_000_000,
          after: 120_000_000,
        }),
        makeRow({
          account_id: "acc-a",
          name: "Alpha Account",
          before: 50_000_000,
          after: 55_000_000,
        }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const ids = screen.getAllByTestId(/^price-movement-row-/).map((el) => el.id);
    expect(ids).toEqual(["price-movement-row-acc-z", "price-movement-row-acc-a"]);
  });

  // PMV-034 — each row's values render in that row's own currency, never converted.
  it("renders each row's values in the row's own currency", () => {
    const report = makeReport({
      rows: [
        makeRow({ account_id: "acc-1", currency: "USD", before: 100_000_000, after: 105_000_000 }),
      ],
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const row = within(screen.getByTestId("price-movement-row-acc-1"));
    expect(row.getByText("100,00 USD")).toBeInTheDocument();
    expect(row.getByText("105,00 USD")).toBeInTheDocument();
  });

  // PMV-033/034 — the total row renders in the report's own total_currency,
  // independent of any row's currency.
  it("renders the total row in total_currency", () => {
    const report = makeReport({
      rows: [makeRow({ account_id: "acc-1", currency: "USD" })],
      total_before: 200_000_000,
      total_after: 220_000_000,
      total_currency: "EUR",
      total_movement_pct: 10_000_000,
    });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    const total = within(screen.getByTestId("price-movement-total"));
    expect(total.getByText("200,00 EUR")).toBeInTheDocument();
    expect(total.getByText("220,00 EUR")).toBeInTheDocument();
    expect(total.getByText("+10,00%")).toBeInTheDocument();
  });

  // PMV-050 — both observation dates present: the value columns are labelled with both.
  it("labels the value columns with both observation dates when both are present", () => {
    const report = makeReport({ observed_from: "2026-09-09", observed_to: "2026-09-11" });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(
      screen.getByText('pmv.dates_range {"from":"2026-09-09","to":"2026-09-11"}'),
    ).toBeInTheDocument();
  });

  // PMV-052 — no prior price: only the date this refresh produced is stated.
  it("labels the value columns with the single date when only observed_to is present", () => {
    const report = makeReport({ observed_from: null, observed_to: "2026-09-11" });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.getByText('pmv.dates_to_only {"date":"2026-09-11"}')).toBeInTheDocument();
  });

  // PMV-051 — the refresh produced no later date: only the earlier date is stated.
  it("labels the value columns with the single date when only observed_from is present", () => {
    const report = makeReport({ observed_from: "2026-09-09", observed_to: null });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.getByText('pmv.dates_from_only {"date":"2026-09-09"}')).toBeInTheDocument();
  });

  // PMV-052 — neither date carried: no date label on the value columns at all.
  it("renders no date label when neither observation date is present", () => {
    const report = makeReport({ observed_from: null, observed_to: null });

    render(<PriceMovementPanel report={report} onDismiss={vi.fn()} />);

    expect(screen.queryByText(/pmv\.dates_/)).not.toBeInTheDocument();
  });
});

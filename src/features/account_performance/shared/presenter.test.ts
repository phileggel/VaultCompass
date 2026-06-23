import { describe, expect, it } from "vitest";
import type { PerformanceMetric, PerformancePeriod } from "@/bindings";
import {
  formatEndValue,
  formatMetricGain,
  formatMetricPct,
  gainColorClass,
  monthLabel,
  pnlColorClass,
  presentAccountPerformanceError,
  presentPeriodRow,
} from "./presenter";

// ---- Helpers ----------------------------------------------------------------

const makeMetric = (overrides: Partial<PerformanceMetric> = {}): PerformanceMetric => ({
  gain: 1_000_000_000, // €1 000.00
  pct: 8_000_000, // 8.00%
  ...overrides,
});

// PRF-070–073 — snapshot column defaults shared by both row factories.
const SNAPSHOT_DEFAULTS = {
  dividends_received: 120_000_000, // €120.00
  realized_pnl: 450_000_000, // €450.00
  unrealized_pnl: -200_000_000, // −€200.00
  cash_balance: 3_000_000_000, // €3 000.00
} satisfies Partial<PerformancePeriod>;

const makeYearRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: null,
  end_value: 10_000_000_000, // €10 000.00
  ...SNAPSHOT_DEFAULTS,
  period_over_period: makeMetric(),
  year_to_date: null, // always null for year rows (PRF-037)
  since_inception: makeMetric({ gain: 2_000_000_000, pct: 20_000_000 }),
  ...overrides,
});

const makeMonthRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: 5,
  end_value: 10_000_000_000,
  ...SNAPSHOT_DEFAULTS,
  period_over_period: makeMetric(),
  year_to_date: makeMetric({ gain: 350_000_000, pct: 3_500_000 }),
  since_inception: makeMetric({ gain: 2_000_000_000, pct: 20_000_000 }),
  ...overrides,
});

// ---- presentAccountPerformanceError (F27, PRF-016, PRF-027) -----------------

describe("presentAccountPerformanceError", () => {
  it("maps AccountNotFound to i18n key (PRF-016)", () => {
    expect(presentAccountPerformanceError({ code: "AccountNotFound", account_id: "x" })).toEqual({
      key: "account_performance.error.account_not_found",
    });
  });

  it("maps DatabaseError to i18n key (PRF-027)", () => {
    expect(presentAccountPerformanceError({ code: "DatabaseError" })).toEqual({
      key: "account_performance.error.database_error",
    });
  });
});

// ---- formatEndValue (PRF-020, micro → display) --------------------------------

describe("formatEndValue", () => {
  it("converts micro-units to display string with 2 decimal places", () => {
    // 10_000_000_000 micros = €10 000.00
    const formatted = formatEndValue(10_000_000_000);
    expect(typeof formatted).toBe("string");
    expect(formatted.length).toBeGreaterThan(0);
  });

  it("formats zero end_value as a zero amount", () => {
    const formatted = formatEndValue(0);
    expect(typeof formatted).toBe("string");
  });
});

// ---- formatMetricGain (PRF-036) -----------------------------------------------

describe("formatMetricGain", () => {
  it("returns formatted gain string when metric is present", () => {
    const formatted = formatMetricGain(makeMetric({ gain: 1_000_000_000 }));
    expect(typeof formatted).toBe("string");
    expect(formatted.length).toBeGreaterThan(0);
  });

  it("returns '—' when metric is null (PRF-042)", () => {
    expect(formatMetricGain(null)).toBe("—");
  });
});

// ---- formatMetricPct (PRF-036, PRF-032) ----------------------------------------

describe("formatMetricPct", () => {
  it("formats micro-percent to display string when pct is present (8_000_000 = 8.00%)", () => {
    const formatted = formatMetricPct(makeMetric({ pct: 8_000_000 }));
    expect(formatted).toContain("8");
    expect(formatted).toContain("%");
  });

  it("returns '—' when pct is null (Dietz denominator was 0, PRF-032)", () => {
    expect(formatMetricPct(makeMetric({ pct: null }))).toBe("—");
  });

  it("returns '—' when the whole metric is null (PRF-042)", () => {
    expect(formatMetricPct(null)).toBe("—");
  });
});

// ---- gainColorClass (PRF-036) -------------------------------------------------

describe("gainColorClass", () => {
  it("returns positive colour class for a positive gain", () => {
    const cls = gainColorClass(makeMetric({ gain: 100_000_000 }));
    expect(cls).toBeTruthy();
    // the class must differ from the negative class
    const negCls = gainColorClass(makeMetric({ gain: -100_000_000 }));
    expect(cls).not.toBe(negCls);
  });

  it("returns negative colour class for a negative gain", () => {
    const cls = gainColorClass(makeMetric({ gain: -500_000_000 }));
    expect(cls).toBeTruthy();
  });

  it("returns neutral colour class for a zero gain", () => {
    const cls = gainColorClass(makeMetric({ gain: 0 }));
    expect(cls).toBeTruthy();
  });

  it("returns neutral colour class when metric is null (PRF-042)", () => {
    const cls = gainColorClass(null);
    expect(cls).toBeTruthy();
  });
});

// ---- pnlColorClass (PRF-071/072) ----------------------------------------------

describe("pnlColorClass", () => {
  it("returns distinct positive and negative classes", () => {
    expect(pnlColorClass(1)).not.toBe(pnlColorClass(-1));
  });

  it("returns a neutral class for zero", () => {
    expect(pnlColorClass(0)).toBe("text-m3-on-surface");
  });
});

// ---- monthLabel (PRF-015) -----------------------------------------------------

describe("monthLabel", () => {
  it("returns an i18n key for a valid month number 1–12", () => {
    const label = monthLabel(1);
    expect(typeof label).toBe("string");
    expect(label.length).toBeGreaterThan(0);
  });

  it("returns distinct labels for different months", () => {
    expect(monthLabel(1)).not.toBe(monthLabel(6));
    expect(monthLabel(6)).not.toBe(monthLabel(12));
  });

  it("covers all twelve months without throwing", () => {
    for (let m = 1; m <= 12; m++) {
      expect(() => monthLabel(m)).not.toThrow();
    }
  });
});

// ---- presentPeriodRow (PRF-036, PRF-037, PRF-041, PRF-042) --------------------

describe("presentPeriodRow — year row", () => {
  it("omits year_to_date from the view model for year rows (PRF-037)", () => {
    const row = presentPeriodRow(makeYearRow());
    // year_to_date must not be present on the year-view row model
    expect(row.yearToDate).toBeUndefined();
  });

  it("includes period_over_period and since_inception for year rows", () => {
    const row = presentPeriodRow(makeYearRow());
    expect(row.periodOverPeriod).toBeDefined();
    expect(row.sinceInception).toBeDefined();
  });

  it("maps the four snapshot columns (PRF-070–073)", () => {
    const row = presentPeriodRow(
      makeYearRow({
        dividends_received: 120_000_000,
        realized_pnl: 450_000_000,
        unrealized_pnl: -200_000_000,
        cash_balance: 3_000_000_000,
      }),
    );
    expect(row.dividendsReceivedFormatted).toBeTruthy();
    expect(row.cashBalanceFormatted).toBeTruthy();
    // realized gain is positive, latent P&L negative → distinct sign colours
    expect(row.realizedPnl.formatted).toBeTruthy();
    expect(row.realizedPnl.colorClass).not.toBe(row.latentPnl.colorClass);
  });

  it("renders '—' for an absent period_over_period (first row, PRF-042)", () => {
    const row = presentPeriodRow(makeYearRow({ period_over_period: null }));
    expect(row.periodOverPeriod.gainFormatted).toBe("—");
    expect(row.periodOverPeriod.pctFormatted).toBe("—");
  });
});

describe("presentPeriodRow — month row", () => {
  it("includes all three metrics for month rows (PRF-036)", () => {
    const row = presentPeriodRow(makeMonthRow());
    expect(row.periodOverPeriod).toBeDefined();
    expect(row.yearToDate).toBeDefined();
    expect(row.sinceInception).toBeDefined();
  });

  it("year_to_date is present on month rows (PRF-034)", () => {
    const row = presentPeriodRow(makeMonthRow());
    expect(row.yearToDate).toBeDefined();
    expect(row.yearToDate?.gainFormatted).not.toBe(undefined);
  });

  it("renders '—' for absent period_over_period on the earliest month row (PRF-042)", () => {
    const row = presentPeriodRow(makeMonthRow({ period_over_period: null }));
    expect(row.periodOverPeriod.gainFormatted).toBe("—");
    expect(row.periodOverPeriod.pctFormatted).toBe("—");
  });

  it("renders '—' for null pct when Dietz denominator was 0 (PRF-032)", () => {
    const row = presentPeriodRow(
      makeMonthRow({
        period_over_period: makeMetric({ gain: 0, pct: null }),
      }),
    );
    expect(row.periodOverPeriod.pctFormatted).toBe("—");
  });

  it("exposes monthLabel for month rows", () => {
    const row = presentPeriodRow(makeMonthRow({ month: 3 }));
    expect(row.periodLabel).toBeTruthy();
  });
});

describe("presentPeriodRow — year label", () => {
  it("exposes the year as the period label for year rows", () => {
    const row = presentPeriodRow(makeYearRow({ year: 2024 }));
    expect(String(row.periodLabel)).toContain("2024");
  });
});

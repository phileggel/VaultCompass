import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PnlCell } from "./PnlCell";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("PnlCell", () => {
  it("applies success color class for a positive value", () => {
    const { container } = render(<PnlCell value="+€100" raw={100} />);
    const span = container.querySelector("span");
    expect(span?.className).toContain("text-m3-success");
    expect(span?.textContent).toBe("+€100");
  });

  it("applies error color class for a negative value", () => {
    const { container } = render(<PnlCell value="-€50" raw={-50} />);
    const span = container.querySelector("span");
    expect(span?.className).toContain("text-m3-error");
    expect(span?.textContent).toBe("-€50");
  });

  it("applies neutral color class and shows placeholder for zero", () => {
    const { container } = render(<PnlCell value="€0" raw={0} />);
    const span = container.querySelector("span");
    expect(span?.className).toContain("text-m3-on-surface-variant");
    expect(span?.textContent).toBe("account_details.pnl_placeholder");
  });
});

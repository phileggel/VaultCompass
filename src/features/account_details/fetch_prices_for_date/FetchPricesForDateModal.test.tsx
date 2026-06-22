import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    fetchAccountAssetPricesForDate: vi.fn(),
  },
}));

const mockShowSnackbar = vi.hoisted(() => vi.fn());
vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

// DateField/FormModal read i18n.language for locale-aware display; provide a stable
// translation context that echoes keys (matches the ClosedHoldingRow test pattern).
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en-US" },
  }),
}));

import * as gateway from "../gateway";
import { FetchPricesForDateModal } from "./FetchPricesForDateModal";

const mockedFetch = vi.mocked(gateway.accountDetailsGateway.fetchAccountAssetPricesForDate);

describe("FetchPricesForDateModal", () => {
  beforeEach(() => vi.clearAllMocks());

  it("renders nothing when closed", () => {
    const { container } = render(
      <FetchPricesForDateModal isOpen={false} onClose={vi.fn()} accountId="account-1" />,
    );
    expect(container.querySelector("#fetch-prices-for-date-form")).toBeNull();
  });

  it("renders the date field and action buttons when open", () => {
    render(<FetchPricesForDateModal isOpen onClose={vi.fn()} accountId="account-1" />);
    expect(document.getElementById("fetch-prices-for-date-date")).not.toBeNull();
    expect(document.getElementById("fetch-prices-for-date-submit")).not.toBeNull();
    expect(document.getElementById("fetch-prices-for-date-cancel")).not.toBeNull();
  });

  it("submitting calls the gateway and closes on success", async () => {
    mockedFetch.mockResolvedValue({ status: "ok", data: { stored: 2, missing: [] } });
    const onClose = vi.fn();
    render(<FetchPricesForDateModal isOpen onClose={onClose} accountId="account-9" />);

    // The submit button is associated to the form via the `form` attribute; jsdom
    // doesn't honour that for implicit submission, so submit the form directly.
    const form = document.getElementById("fetch-prices-for-date-form");
    if (form === null) throw new Error("form not rendered");
    fireEvent.submit(form);

    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(1));
    expect(mockedFetch).toHaveBeenCalledWith(
      "account-9",
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("cancel button invokes onClose without fetching", async () => {
    const onClose = vi.fn();
    render(<FetchPricesForDateModal isOpen onClose={onClose} accountId="account-1" />);

    await userEvent.click(screen.getByText("action.cancel"));

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(mockedFetch).not.toHaveBeenCalled();
  });
});

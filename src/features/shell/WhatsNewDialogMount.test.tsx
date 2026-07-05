import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WhatsNewDialogMount } from "./WhatsNewDialogMount";

const { mockUseWhatsNewDialog, mockDismiss } = vi.hoisted(() => ({
  mockUseWhatsNewDialog: vi.fn(),
  mockDismiss: vi.fn(),
}));

vi.mock("@/features/whats_new/useWhatsNewDialog", () => ({
  useWhatsNewDialog: (changelogText: string) => mockUseWhatsNewDialog(changelogText),
}));

// Stub the dialog so the mount renders in isolation; surface its props.
vi.mock("@/features/whats_new/WhatsNewDialog", () => ({
  WhatsNewDialog: ({
    version,
    sections,
    onDismiss,
  }: {
    version: string;
    sections: { version: string }[];
    onDismiss: () => void;
  }) => (
    <button type="button" data-testid="whats-new-dialog" onClick={onDismiss}>
      {version}:{sections.length}
    </button>
  ),
}));

vi.mock("@/lib/logger", () => ({ logger: { info: vi.fn(), error: vi.fn() } }));

describe("WhatsNewDialogMount (WNW-020/030/050)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders nothing while the hook reports no sections (WNW-030)", () => {
    mockUseWhatsNewDialog.mockReturnValue({
      appVersion: "0.33.0",
      sections: null,
      dismiss: mockDismiss,
    });
    const { container } = render(<WhatsNewDialogMount />);
    expect(container).toBeEmptyDOMElement();
  });

  it("feeds the bundled changelog text into the hook", () => {
    mockUseWhatsNewDialog.mockReturnValue({
      appVersion: "0.33.0",
      sections: null,
      dismiss: mockDismiss,
    });
    render(<WhatsNewDialogMount />);
    const changelogText = mockUseWhatsNewDialog.mock.calls[0]?.[0];
    expect(changelogText).toContain("Changelog");
  });

  it("renders the dialog with the resolved version and sections when visible (WNW-020)", () => {
    mockUseWhatsNewDialog.mockReturnValue({
      appVersion: "0.33.0",
      sections: [{ version: "0.33.0" }, { version: "0.32.0" }],
      dismiss: mockDismiss,
    });
    render(<WhatsNewDialogMount />);
    expect(screen.getByTestId("whats-new-dialog")).toHaveTextContent("0.33.0:2");
  });

  it("wires the dialog's dismiss back to the hook (WNW-050)", () => {
    mockUseWhatsNewDialog.mockReturnValue({
      appVersion: "0.33.0",
      sections: [{ version: "0.33.0" }],
      dismiss: mockDismiss,
    });
    render(<WhatsNewDialogMount />);
    fireEvent.click(screen.getByTestId("whats-new-dialog"));
    expect(mockDismiss).toHaveBeenCalledTimes(1);
  });
});

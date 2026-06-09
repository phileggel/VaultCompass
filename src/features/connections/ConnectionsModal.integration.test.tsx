import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the connections gateway at the test boundary (F3 — components must not
// call commands.* directly; the gateway is the contract boundary here).
vi.mock("./gateway");

// Mock i18n so t(key) returns the key — tests assert on i18n keys, not literal labels (F24)
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
  Trans: ({ i18nKey }: { i18nKey: string }) => i18nKey,
}));

// Mock snackbar store
const mockShowSnackbar = vi.hoisted(() => vi.fn());
vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

import { ConnectionsModal } from "./ConnectionsModal";
import * as gateway from "./gateway";

describe("ConnectionsModal — provider list (KEY-031)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-016 / KEY-031 — "No key" state: row shows no-key status when has_key is false
  it("renders a no-key status row when provider has no key stored", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    expect(await screen.findByTestId("provider-row-Stooq")).toBeInTheDocument();
    expect(await screen.findByTestId("provider-status-Stooq")).toHaveTextContent(
      "connection.status.no_key",
    );
  });

  // KEY-016 / KEY-015 — "Key set" state: row shows key-set status + tier label
  it("renders key-set status and tier label when provider has a key stored", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    expect(await screen.findByTestId("provider-status-Stooq")).toHaveTextContent(
      "connection.status.key_set",
    );
    expect(await screen.findByTestId("provider-tier-Stooq")).toHaveTextContent(
      "connection.tier.os_keychain",
    );
  });

  // KEY-015 — tier label for SessionMemory
  it("renders session-memory tier label when active_tier is SessionMemory", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "SessionMemory" }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    expect(await screen.findByTestId("provider-tier-Stooq")).toHaveTextContent(
      "connection.tier.session_memory",
    );
  });

  // KeyStoreError on load — error state rendered inline (not a crash)
  it("renders inline error when getProviderConnections returns KeyStoreError", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "error",
      error: { code: "KeyStoreError" },
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    expect(await screen.findByTestId("connections-load-error")).toBeInTheDocument();
  });
});

describe("ConnectionsModal — provider row: Test action (KEY-020)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-020 — Test button is disabled when the key field is empty
  it("Test button is disabled when key input is empty", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const testButton = await screen.findByTestId("provider-test-btn-Stooq");
    expect(testButton).toBeDisabled();
  });

  // KEY-020 — Test button is enabled when key field has a non-empty value
  it("Test button is enabled when key input contains a non-empty value", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.testProviderKey).mockResolvedValue({
      status: "ok",
      data: "Accepted",
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "my-api-key");

    const testButton = screen.getByTestId("provider-test-btn-Stooq");
    expect(testButton).toBeEnabled();
  });
});

describe("ConnectionsModal — provider row: Test outcomes (KEY-023)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-023 — Accepted outcome shows accepted feedback
  it("shows accepted feedback when test returns Accepted", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.testProviderKey).mockResolvedValue({
      status: "ok",
      data: "Accepted",
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "valid-key");
    await userEvent.click(screen.getByTestId("provider-test-btn-Stooq"));

    expect(await screen.findByTestId("provider-test-outcome-Stooq")).toHaveTextContent(
      "connection.test_outcome.accepted",
    );
  });

  // KEY-023 — Rejected outcome shows rejected feedback
  it("shows rejected feedback when test returns Rejected", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.testProviderKey).mockResolvedValue({
      status: "ok",
      data: "Rejected",
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "bad-key");
    await userEvent.click(screen.getByTestId("provider-test-btn-Stooq"));

    expect(await screen.findByTestId("provider-test-outcome-Stooq")).toHaveTextContent(
      "connection.test_outcome.rejected",
    );
  });

  // KEY-023 — Unreachable outcome shows unreachable feedback
  it("shows unreachable feedback when test returns Unreachable", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.testProviderKey).mockResolvedValue({
      status: "ok",
      data: "Unreachable",
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "some-key");
    await userEvent.click(screen.getByTestId("provider-test-btn-Stooq"));

    expect(await screen.findByTestId("provider-test-outcome-Stooq")).toHaveTextContent(
      "connection.test_outcome.unreachable",
    );
  });
});

describe("ConnectionsModal — provider row: Save (KEY-033)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-033 — successful save updates row to key-set state and shows snackbar
  it("calls saveProviderKey and shows save-success snackbar on success", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.saveProviderKey).mockResolvedValue({
      status: "ok",
      data: { provider: "Stooq", has_key: true, active_tier: "OsKeychain" },
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "my-new-key");
    await userEvent.click(screen.getByTestId("provider-save-btn-Stooq"));

    expect(gateway.connectionGateway.saveProviderKey).toHaveBeenCalledWith(
      expect.objectContaining({ provider: "Stooq", key: "my-new-key" }),
    );
    expect(mockShowSnackbar).toHaveBeenCalledWith("connection.save_success", expect.any(String));
  });

  // KEY-033 — EmptyKey error shows inline error in the row
  it("shows inline error when save returns EmptyKey", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.saveProviderKey).mockResolvedValue({
      status: "error",
      error: { code: "EmptyKey" },
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, " ");
    await userEvent.click(screen.getByTestId("provider-save-btn-Stooq"));

    expect(await screen.findByTestId("provider-save-error-Stooq")).toHaveTextContent(
      "connection.error.empty_key",
    );
  });
});

describe("ConnectionsModal — provider row: Remove (KEY-034)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-034 — Remove button absent when has_key is false
  it("does not render Remove button when provider has no key", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    await screen.findByTestId("provider-row-Stooq");
    expect(screen.queryByTestId("provider-remove-btn-Stooq")).not.toBeInTheDocument();
  });

  // KEY-034 — Remove button present when has_key is true
  it("renders Remove button when provider has a key stored", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    expect(await screen.findByTestId("provider-remove-btn-Stooq")).toBeInTheDocument();
  });

  // KEY-034 — Remove requires confirmation before calling gateway
  it("calls removeProviderKey after confirmation and shows remove-success snackbar", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });
    vi.mocked(gateway.connectionGateway.removeProviderKey).mockResolvedValue({
      status: "ok",
      data: null,
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    await userEvent.click(await screen.findByTestId("provider-remove-btn-Stooq"));

    // Confirmation dialog must appear before the gateway is called
    expect(screen.getByTestId("remove-confirm-dialog")).toBeInTheDocument();
    expect(gateway.connectionGateway.removeProviderKey).not.toHaveBeenCalled();

    await userEvent.click(screen.getByTestId("remove-confirm-ok"));

    expect(gateway.connectionGateway.removeProviderKey).toHaveBeenCalledWith(
      expect.objectContaining({ provider: "Stooq" }),
    );
    expect(mockShowSnackbar).toHaveBeenCalledWith("connection.remove_success", expect.any(String));
  });
});

describe("ConnectionsModal — in-flight state (KEY-035)", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-035 — save button disabled while save is in flight
  it("disables save button while saveProviderKey is in progress", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    let resolveSave!: (v: {
      status: "ok";
      data: { provider: "Stooq"; has_key: true; active_tier: "OsKeychain" };
    }) => void;
    vi.mocked(gateway.connectionGateway.saveProviderKey).mockReturnValue(
      new Promise((resolve) => {
        resolveSave = resolve;
      }),
    );

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "key");
    await userEvent.click(screen.getByTestId("provider-save-btn-Stooq"));

    expect(screen.getByTestId("provider-save-btn-Stooq")).toBeDisabled();

    resolveSave({
      status: "ok",
      data: { provider: "Stooq", has_key: true, active_tier: "OsKeychain" },
    });
  });

  // KEY-035 — test button disabled while testProviderKey is in progress
  it("disables test button while testProviderKey is in progress", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    let resolveTest!: (v: { status: "ok"; data: "Accepted" }) => void;
    vi.mocked(gateway.connectionGateway.testProviderKey).mockReturnValue(
      new Promise((resolve) => {
        resolveTest = resolve;
      }),
    );

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "key");
    await userEvent.click(screen.getByTestId("provider-test-btn-Stooq"));

    expect(screen.getByTestId("provider-test-btn-Stooq")).toBeDisabled();

    resolveTest({ status: "ok", data: "Accepted" });
  });
});

describe("ConnectionsModal — KEY-012 plaintext opt-in", () => {
  beforeEach(() => vi.clearAllMocks());

  // KEY-012 — a save that lands in session memory (no keychain) offers the
  // persistent plaintext tier; taking it re-saves with allow_plaintext: true.
  it("offers the plaintext opt-in on a session-memory save and re-saves with allow_plaintext", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.saveProviderKey).mockResolvedValue({
      status: "ok",
      data: { provider: "Stooq", has_key: true, active_tier: "SessionMemory" },
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "my-key");
    await userEvent.click(screen.getByTestId("provider-save-btn-Stooq"));

    await userEvent.click(await screen.findByTestId("provider-plaintext-optin-Stooq"));

    expect(gateway.connectionGateway.saveProviderKey).toHaveBeenLastCalledWith(
      expect.objectContaining({ provider: "Stooq", allow_plaintext: true }),
    );
  });

  // KEY-012 — a save into the OS keychain does NOT offer the plaintext opt-in.
  it("does not offer the plaintext opt-in when the save lands in the OS keychain", async () => {
    vi.mocked(gateway.connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });
    vi.mocked(gateway.connectionGateway.saveProviderKey).mockResolvedValue({
      status: "ok",
      data: { provider: "Stooq", has_key: true, active_tier: "OsKeychain" },
    });

    render(<ConnectionsModal open={true} onClose={vi.fn()} />);

    const keyInput = await screen.findByTestId("provider-key-input-Stooq");
    await userEvent.type(keyInput, "my-key");
    await userEvent.click(screen.getByTestId("provider-save-btn-Stooq"));

    await screen.findByTestId("provider-row-Stooq");
    expect(screen.queryByTestId("provider-plaintext-optin-Stooq")).not.toBeInTheDocument();
  });
});

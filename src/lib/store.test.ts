import { beforeEach, describe, expect, it, vi } from "vitest";

const mockDebug = vi.hoisted(() => vi.fn());

// Mock all gateways that store.ts imports so the module loads cleanly in tests
vi.mock("../features/accounts/gateway", () => ({
  accountGateway: { getAccounts: vi.fn().mockResolvedValue({ status: "ok", data: [] }) },
}));

vi.mock("../features/assets/gateway", () => ({
  assetGateway: {
    getAssetsWithArchived: vi.fn().mockResolvedValue({ status: "ok", data: [] }),
  },
}));

vi.mock("../features/categories/gateway", () => ({
  categoryGateway: { getCategories: vi.fn().mockResolvedValue({ status: "ok", data: [] }) },
}));

vi.mock("./logger", () => ({
  logger: { error: vi.fn(), info: vi.fn(), debug: mockDebug },
}));

// Capture the events.event.listen callback so tests can fire synthetic events
let capturedEventListener: ((event: { payload: { type: string } }) => void) | null = null;

vi.mock("../bindings", () => ({
  events: {
    event: {
      listen: vi.fn((cb: (event: { payload: { type: string } }) => void) => {
        capturedEventListener = cb;
        return Promise.resolve(() => {});
      }),
    },
  },
}));

vi.mock("@tauri-apps/api/app", () => ({
  getName: vi.fn().mockResolvedValue("VaultCompass"),
  getVersion: vi.fn().mockResolvedValue("0.0.0"),
}));

const { useAppStore } = await import("./store");

describe("store — locallyHandledEvents (FXR-037)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedEventListener = null;
    // Reset store to un-initialized so init() runs fresh
    useAppStore.setState({ isInitialized: false });
  });

  // FXR-037 — CurrencyRateUpdated must be in locallyHandledEvents so it does NOT
  // trigger a global re-fetch AND does not emit an "unhandled event" debug log.
  // The test fires the event and asserts debug was NOT called (if it were truly
  // unhandled, logger.debug("[store] unhandled event", ...) would fire).
  it("FXR-037 — CurrencyRateUpdated is listed as locally handled (no debug log emitted)", async () => {
    const cleanup = useAppStore.getState().init();
    await new Promise((r) => setTimeout(r, 0));

    capturedEventListener?.({ payload: { type: "CurrencyRateUpdated" } });

    // If CurrencyRateUpdated is NOT in locallyHandledEvents, store.ts will call
    // logger.debug("[store] unhandled event", ...) — that is the failure signal.
    expect(mockDebug).not.toHaveBeenCalledWith(
      "[store] unhandled event",
      expect.objectContaining({ type: "CurrencyRateUpdated" }),
    );

    cleanup();
  });

  // Tighter assertion: CurrencyRateUpdated must not trigger any global gateway call
  it("FXR-037 — CurrencyRateUpdated does not trigger fetchAssets / fetchAccounts / fetchCategories", async () => {
    const cleanup = useAppStore.getState().init();
    await new Promise((r) => setTimeout(r, 0));

    const { assetGateway } = await import("../features/assets/gateway");
    const { accountGateway } = await import("../features/accounts/gateway");
    const { categoryGateway } = await import("../features/categories/gateway");

    const assetCallsBefore = vi.mocked(assetGateway.getAssetsWithArchived).mock.calls.length;
    const accountCallsBefore = vi.mocked(accountGateway.getAccounts).mock.calls.length;
    const categoryCallsBefore = vi.mocked(categoryGateway.getCategories).mock.calls.length;

    capturedEventListener?.({ payload: { type: "CurrencyRateUpdated" } });

    expect(vi.mocked(assetGateway.getAssetsWithArchived).mock.calls.length).toBe(assetCallsBefore);
    expect(vi.mocked(accountGateway.getAccounts).mock.calls.length).toBe(accountCallsBefore);
    expect(vi.mocked(categoryGateway.getCategories).mock.calls.length).toBe(categoryCallsBefore);

    cleanup();
  });

  // Sanity: AssetUpdated still triggers fetchAssets (regression guard)
  it("AssetUpdated still triggers fetchAssets (regression guard for FXR-037 scope)", async () => {
    const cleanup = useAppStore.getState().init();
    await new Promise((r) => setTimeout(r, 0));

    const { assetGateway } = await import("../features/assets/gateway");
    const callsBefore = vi.mocked(assetGateway.getAssetsWithArchived).mock.calls.length;

    capturedEventListener?.({ payload: { type: "AssetUpdated" } });

    expect(vi.mocked(assetGateway.getAssetsWithArchived).mock.calls.length).toBeGreaterThan(
      callsBefore,
    );

    cleanup();
  });
});

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FeeGenerationError, PortfolioSyncError, Result, SyncStatus } from "@/bindings";
import { commands, events } from "@/bindings";

// SYN-063 — the shell indicator reads the sync status through its own gateway (F26).
export function getSyncStatus(): Promise<Result<SyncStatus, PortfolioSyncError>> {
  return commands.getSyncStatus();
}

// SYN-064 — a run that applied changes or changed the device's state has completed.
export function onSyncCompleted(callback: () => void): Promise<UnlistenFn> {
  return events.event.listen((event) => {
    if (event.payload.type === "SyncCompleted") {
      callback();
    }
  });
}

export const shellGateway = {
  onMigrationError(cb: (message: string) => void): Promise<UnlistenFn> {
    return listen<string>("db:migration_error", (event) => cb(event.payload));
  },

  // FEE-040 — apply every due recurring fee deduction (lazy catch-up on app start).
  applyDueFeeDeductions(): Promise<Result<null, FeeGenerationError>> {
    return commands.applyDueFeeDeductions();
  },

  getSyncStatus,
  onSyncCompleted,
};

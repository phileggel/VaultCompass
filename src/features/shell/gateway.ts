import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FeeGenerationError, Result } from "@/bindings";
import { commands } from "@/bindings";

export const shellGateway = {
  onMigrationError(cb: (message: string) => void): Promise<UnlistenFn> {
    return listen<string>("db:migration_error", (event) => cb(event.payload));
  },

  // FEE-040 — apply every due recurring fee deduction (lazy catch-up on app start).
  applyDueFeeDeductions(): Promise<Result<null, FeeGenerationError>> {
    return commands.applyDueFeeDeductions();
  },
};

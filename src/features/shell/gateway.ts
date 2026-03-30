import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const shellGateway = {
  onMigrationError(cb: (message: string) => void): Promise<UnlistenFn> {
    return listen<string>("db:migration_error", (event) => cb(event.payload));
  },
};

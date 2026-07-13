import {
  commands,
  type Result,
  type ScheduledFetchError,
  type ScheduledFetchStatus,
} from "../../bindings";

/**
 * Gateway for the Settings feature's scheduled-fetch commands (SPF-010–061).
 * Centralizes all Tauri command calls for the feature (the only file allowed
 * to touch `commands.*`, F3). Each function is a typed `Result` pass-through
 * (F27) matching the `bindings.ts` signature.
 */
export async function configureScheduledFetch(
  enabled: boolean,
  triggerTime: string,
): Promise<Result<null, ScheduledFetchError>> {
  return await commands.configureScheduledFetch(enabled, triggerTime);
}

export async function getScheduledFetchStatus(): Promise<
  Result<ScheduledFetchStatus, ScheduledFetchError>
> {
  return await commands.getScheduledFetchStatus();
}

export const settingsGateway = {
  configureScheduledFetch,
  getScheduledFetchStatus,
};

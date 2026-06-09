import {
  type ConnectionError,
  commands,
  type ProviderConnection,
  type ProviderKeyTestOutcome,
  type RemoveProviderKeyArgs,
  type Result,
  type SaveProviderKeyArgs,
  type TestProviderKeyArgs,
} from "../../bindings";

/**
 * Gateway for Connection (provider API-key) backend communication.
 * The only file in the feature allowed to call `commands.*` (F3); it passes the
 * typed `Result` through unchanged (F27 — never throws, never unwraps).
 */
export const connectionGateway = {
  async getProviderConnections(): Promise<Result<ProviderConnection[], ConnectionError>> {
    return await commands.getProviderConnections();
  },

  async saveProviderKey(
    args: SaveProviderKeyArgs,
  ): Promise<Result<ProviderConnection, ConnectionError>> {
    return await commands.saveProviderKey(args);
  },

  async testProviderKey(
    args: TestProviderKeyArgs,
  ): Promise<Result<ProviderKeyTestOutcome, ConnectionError>> {
    return await commands.testProviderKey(args);
  },

  async removeProviderKey(args: RemoveProviderKeyArgs): Promise<Result<null, ConnectionError>> {
    return await commands.removeProviderKey(args);
  },
};

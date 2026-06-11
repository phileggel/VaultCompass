import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/bindings";
import { Button } from "@/ui/components/button/Button";
import { TextField } from "@/ui/components/field/TextField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { connectionGateway } from "./gateway";
import { storageTierToLabel } from "./shared/presenter";
import { useProviderRow } from "./useProviderRow";

interface ConnectionsModalProps {
  open: boolean;
  onClose: () => void;
}

/** Where the user obtains a free key, per provider (KEY-032 signup link). */
const SIGNUP_URL: Record<string, string> = {
  Stooq: "https://stooq.com/q/d/?s=spy.us&get_apikey",
};

/**
 * Connections dialog (KEY-030/031): lists supported providers (Stooq in v1) and
 * lets the user paste, test, and remove a BYOK API key per provider.
 */
export function ConnectionsModal({ open, onClose }: ConnectionsModalProps) {
  const { t } = useTranslation();
  const [connections, setConnections] = useState<ProviderConnection[]>([]);
  const [loadError, setLoadError] = useState(false);

  const load = useCallback(async () => {
    const result = await connectionGateway.getProviderConnections();
    if (result.status === "ok") {
      setConnections(result.data);
      setLoadError(false);
    } else {
      setLoadError(true);
    }
  }, []);

  useEffect(() => {
    if (open) void load();
  }, [open, load]);

  return (
    <FormModal isOpen={open} onClose={onClose} title={t("connection.title")}>
      {loadError ? (
        <div data-testid="connections-load-error" className="text-sm text-m3-error">
          {t("connection.error.key_store_error")}
        </div>
      ) : (
        connections.map((connection) => (
          <ProviderRow key={connection.provider} connection={connection} onMutated={load} />
        ))
      )}
    </FormModal>
  );
}

interface ProviderRowProps {
  connection: ProviderConnection;
  onMutated: () => Promise<void>;
}

function ProviderRow({ connection, onMutated }: ProviderRowProps) {
  const { t } = useTranslation();
  const provider = connection.provider;
  const {
    apiKey,
    setApiKey,
    testing,
    saving,
    removing,
    outcome,
    testError,
    saveError,
    confirmingRemove,
    setConfirmingRemove,
    offerPlaintext,
    handleTest,
    handleSave,
    handleRemove,
  } = useProviderRow(provider, onMutated);

  return (
    <div
      data-testid={`provider-row-${provider}`}
      className="flex flex-col gap-2 rounded-2xl border border-m3-outline-variant p-4"
    >
      <div className="flex items-center justify-between">
        <span className="font-medium text-m3-on-surface">{provider}</span>
        <span
          data-testid={`provider-status-${provider}`}
          className="text-sm text-m3-on-surface-variant"
        >
          {t(connection.has_key ? "connection.status.key_set" : "connection.status.no_key")}
        </span>
      </div>

      {connection.has_key && connection.active_tier && (
        <span
          data-testid={`provider-tier-${provider}`}
          className="text-xs text-m3-on-surface-variant"
        >
          {t(storageTierToLabel(connection.active_tier))}
        </span>
      )}

      {/* External URL must open in the system browser via plugin-opener, never
          inside the WebView (in-WebView `target="_blank"` behavior is
          platform-dependent and uncontrolled). */}
      <button
        type="button"
        id={`provider-${provider}-signup`}
        onClick={() => {
          const url = SIGNUP_URL[provider];
          if (url) void openUrl(url);
        }}
        className="w-fit text-xs text-m3-primary underline"
      >
        {t("connection.signup_link")}
      </button>

      <TextField
        id={`provider-key-input-${provider}`}
        data-testid={`provider-key-input-${provider}`}
        label={t("connection.key_input_label")}
        type="password"
        value={apiKey}
        onChange={(e) => setApiKey(e.target.value)}
        placeholder={t("connection.key_placeholder")}
      />

      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          id={`provider-${provider}-test`}
          data-testid={`provider-test-btn-${provider}`}
          disabled={apiKey.trim() === "" || testing}
          onClick={handleTest}
        >
          {t("connection.action.test")}
        </Button>
        <Button
          variant="primary"
          size="sm"
          id={`provider-${provider}-save`}
          data-testid={`provider-save-btn-${provider}`}
          disabled={saving}
          onClick={() => handleSave()}
        >
          {t("connection.action.save")}
        </Button>
        {connection.has_key && (
          <Button
            variant="ghost"
            size="sm"
            id={`provider-${provider}-remove`}
            data-testid={`provider-remove-btn-${provider}`}
            disabled={removing}
            onClick={() => setConfirmingRemove(true)}
          >
            {t("connection.action.remove")}
          </Button>
        )}
      </div>

      {outcome && (
        <span data-testid={`provider-test-outcome-${provider}`} className="text-sm">
          {t(`connection.test_outcome.${outcome}`)}
        </span>
      )}

      {testError && (
        <span data-testid={`provider-test-error-${provider}`} className="text-sm text-m3-error">
          {t(testError)}
        </span>
      )}

      {saveError && (
        <span data-testid={`provider-save-error-${provider}`} className="text-sm text-m3-error">
          {t(saveError)}
        </span>
      )}

      {offerPlaintext && (
        <div className="flex flex-col gap-2 rounded-xl bg-m3-surface-container p-3">
          <span className="text-sm text-m3-on-surface-variant">
            {t("connection.plaintext_optin_prompt")}
          </span>
          <Button
            variant="tonal"
            size="sm"
            id={`provider-${provider}-plaintext`}
            data-testid={`provider-plaintext-optin-${provider}`}
            disabled={saving}
            onClick={() => handleSave(true)}
          >
            {t("connection.action.save_to_disk")}
          </Button>
        </div>
      )}

      {confirmingRemove && (
        <div
          data-testid="remove-confirm-dialog"
          className="flex flex-col gap-2 rounded-xl bg-m3-surface-container p-3"
        >
          <span className="text-sm text-m3-on-surface">
            {t("connection.remove_confirm", { provider })}
          </span>
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              size="sm"
              disabled={removing}
              onClick={() => setConfirmingRemove(false)}
            >
              {t("action.cancel")}
            </Button>
            <Button
              variant="danger"
              size="sm"
              id="remove-confirm-ok"
              data-testid="remove-confirm-ok"
              disabled={removing}
              onClick={handleRemove}
            >
              {t("connection.action.remove")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

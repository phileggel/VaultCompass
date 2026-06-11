import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Provider } from "@/bindings";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { connectionGateway } from "./gateway";
import {
  connectionErrorToI18n,
  type TestOutcomeUiState,
  testOutcomeToUiState,
} from "./shared/presenter";

/**
 * Per-provider row state + actions for the Connections dialog (KEY-020/023/033/
 * 034/035). Owns the key input, test/save/remove flows, in-flight flags, inline
 * errors (F27), and the KEY-012 plaintext opt-in offered when a save lands in the
 * session-memory tier (no OS keychain available).
 */
export function useProviderRow(provider: Provider, onMutated: () => Promise<void>) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();

  const [apiKey, setApiKey] = useState("");
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [outcome, setOutcome] = useState<TestOutcomeUiState | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);
  const [confirmingRemove, setConfirmingRemove] = useState(false);
  // KEY-012 — set when a save fell back to session memory (keychain unavailable),
  // offering the user the persistent (opt-in) plaintext tier.
  const [offerPlaintext, setOfferPlaintext] = useState(false);

  const handleTest = useCallback(async () => {
    setTesting(true);
    setOutcome(null);
    setTestError(null);
    try {
      const result = await connectionGateway.testProviderKey({ provider, key: apiKey });
      if (result.status === "ok") {
        setOutcome(testOutcomeToUiState(result.data));
      } else {
        setTestError(connectionErrorToI18n(result.error));
      }
    } finally {
      setTesting(false);
    }
  }, [provider, apiKey]);

  const handleSave = useCallback(
    async (allowPlaintext = false) => {
      setSaving(true);
      setSaveError(null);
      try {
        const result = await connectionGateway.saveProviderKey({
          provider,
          key: apiKey,
          allow_plaintext: allowPlaintext,
        });
        if (result.status !== "ok") {
          setSaveError(connectionErrorToI18n(result.error));
          return;
        }
        showSnackbar(t("connection.save_success"), "success");
        // KEY-012 — landed in session memory ⇒ no keychain; offer persistent
        // plaintext as an explicit opt-in (unless the user just took it).
        if (result.data.active_tier === "SessionMemory" && !allowPlaintext) {
          setOfferPlaintext(true);
        } else {
          setOfferPlaintext(false);
          setApiKey("");
          await onMutated();
        }
      } finally {
        setSaving(false);
      }
    },
    [provider, apiKey, showSnackbar, t, onMutated],
  );

  const handleRemove = useCallback(async () => {
    setRemoving(true);
    try {
      const result = await connectionGateway.removeProviderKey({ provider });
      if (result.status === "ok") {
        showSnackbar(t("connection.remove_success"), "success");
        setConfirmingRemove(false);
        await onMutated();
      } else {
        // F27 — surface the failure rather than silently dropping it.
        showSnackbar(t(connectionErrorToI18n(result.error)), "error");
        setConfirmingRemove(false);
      }
    } finally {
      setRemoving(false);
    }
  }, [provider, showSnackbar, t, onMutated]);

  return {
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
  };
}

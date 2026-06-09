import { useNavigate, useSearch } from "@tanstack/react-router";
import { useCallback } from "react";
import { ConnectionsModal } from "@/features/connections/ConnectionsModal";
import { patchModalSearch } from "@/lib/modalSearch";

/**
 * Shell-level URL-driven mount for the Connections dialog (KEY-030). Renders the
 * dialog when `?modal=connections` is present in the URL; closing clears the param.
 * Lets the side-menu entry and the price-refresh key gate (KEY-040) open the dialog
 * by mutating URL params only — no cross-feature import at the call site.
 */
export function ConnectionsModalMount() {
  const search = useSearch({ strict: false }) as Record<string, unknown>;
  const navigate = useNavigate();
  const modal = typeof search.modal === "string" ? search.modal : undefined;

  const handleClose = useCallback(() => {
    patchModalSearch(navigate, { modal: undefined }, { replace: true });
  }, [navigate]);

  if (modal !== "connections") return null;
  return <ConnectionsModal open={true} onClose={handleClose} />;
}

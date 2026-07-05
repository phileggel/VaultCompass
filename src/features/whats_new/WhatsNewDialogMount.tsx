import { useEffect } from "react";
import { logger } from "@/lib/logger";
import changelogText from "../../../CHANGELOG.md?raw";
import { useWhatsNewDialog } from "./useWhatsNewDialog";
import { WhatsNewDialog } from "./WhatsNewDialog";

/**
 * Shell-level mount for the What's-new dialog (WNW-020). Once the app version
 * resolves, compares it against the last acknowledged version and overlays the
 * changelog sections released in between. Fresh installs seed silently and render
 * nothing (WNW-030).
 */
export function WhatsNewDialogMount() {
  const { appVersion, sections, dismiss } = useWhatsNewDialog(changelogText);

  useEffect(() => {
    logger.info("[WhatsNewDialogMount] mounted");
  }, []);

  if (!sections) return null;

  return <WhatsNewDialog version={appVersion} sections={sections} onDismiss={dismiss} />;
}

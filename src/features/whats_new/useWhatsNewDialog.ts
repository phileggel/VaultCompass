import { useCallback, useEffect, useState } from "react";
import { useAppStore } from "@/lib/store";
import { getWhatsNewLastSeenVersion, setWhatsNewLastSeenVersion } from "@/lib/whatsNewStorage";
import { type ChangelogSection, extractSectionsBetween } from "./parseChangelog";

/** The store's initial appVersion before getVersion() resolves. */
const UNRESOLVED_APP_VERSION = "...";

/**
 * Decides whether the What's-new dialog opens on this launch (WNW-020) and holds the
 * changelog sections to show. Waits for the app version to resolve; a fresh install
 * (no stored last-seen version) seeds silently and shows nothing (WNW-030), as does a
 * changelog with no matching sections (WNW-070). Dismissing acknowledges the current
 * version (WNW-050).
 */
export function useWhatsNewDialog(changelogText: string) {
  const appVersion = useAppStore((state) => state.appVersion);
  const [sections, setSections] = useState<ChangelogSection[] | null>(null);

  useEffect(() => {
    if (appVersion === UNRESOLVED_APP_VERSION) return;
    const lastSeenVersion = getWhatsNewLastSeenVersion();
    if (lastSeenVersion === appVersion) return;
    if (lastSeenVersion === null) {
      setWhatsNewLastSeenVersion(appVersion);
      return;
    }
    const found = extractSectionsBetween(changelogText, lastSeenVersion, appVersion);
    if (found.length === 0) {
      setWhatsNewLastSeenVersion(appVersion);
      return;
    }
    setSections(found);
  }, [appVersion, changelogText]);

  const dismiss = useCallback(() => {
    setWhatsNewLastSeenVersion(useAppStore.getState().appVersion);
    setSections(null);
  }, []);

  return { appVersion, sections, dismiss };
}

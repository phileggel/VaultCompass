/**
 * Format an ISO date (YYYY-MM-DD) as a locale-numeric date — e.g. `14/06/2026`
 * for `fr`, `6/14/2026` for `en`. Anchored at noon so the rendered day never
 * shifts under a timezone offset. Returns the raw input unchanged if it does not
 * parse. Lives under `ui/format/` as a cross-feature primitive (not feature-owned).
 */
export function formatIsoDateNumeric(isoDate: string, locale: string): string {
  const date = new Date(`${isoDate}T12:00:00`);
  return Number.isNaN(date.getTime()) ? isoDate : new Intl.DateTimeFormat(locale).format(date);
}

/**
 * Converts ISO date to the DateField display format.
 * DateField defaults to fr-FR locale → DD/MM/YYYY. (E2E rule E7)
 */
export function isoToDisplayDate(iso: string): string {
  const [year, month, day] = iso.split("-");
  return `${day}/${month}/${year}`; // "2020-01-15" → "15/01/2020"
}

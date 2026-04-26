/**
 * Micro-unit conversion utilities (ADR-001, TRX-024).
 *
 * All financial values are stored and transmitted as i64 micro-units (value × 1_000_000).
 * Decimal ↔ micro conversion occurs ONLY at the UI boundary:
 *   - User input:  decimal string → number (micro-units) via decimalToMicro
 *   - Display:     number (micro-units) → formatted decimal string via microToDecimal
 *
 * All internal calculations are performed on micro-unit integers (computeTotalMicro).
 */

const MICRO = 1_000_000;

/**
 * Converts a decimal string to an integer micro-unit value.
 * e.g. "1.5" → 1_500_000
 * Returns 0 for empty, invalid, or non-numeric input.
 *
 * Parses integer and fractional parts separately to avoid IEEE-754 rounding errors.
 */
export function decimalToMicro(value: string): number {
  const trimmed = value.trim().replace(",", ".");
  if (!trimmed || Number.isNaN(Number(trimmed))) return 0;
  const [intStr, fracStr = ""] = trimmed.split(".");
  const intPart = Number.parseInt(intStr || "0", 10);
  const fracPadded = fracStr.padEnd(6, "0").slice(0, 6);
  const fracPart = Number.parseInt(fracPadded, 10);
  return intPart * MICRO + fracPart;
}

/**
 * Converts an integer micro-unit value to a plain decimal string using a period separator.
 * Use for form pre-fill only — not locale-aware.
 * e.g. 1_500_000 → "1.500" (3 decimal places by default per TRX-024)
 */
export function microToDecimal(micros: number, decimals = 3): string {
  return (micros / MICRO).toFixed(decimals);
}

/**
 * Converts an integer micro-unit value to a locale-aware display string.
 * Use for read-only display in tables and labels — never for editable inputs.
 * e.g. 1_500_000 → "1.500" (en-US) or "1,500" (fr-FR) with 3 decimal places
 */
export function microToFormatted(micros: number, decimals = 3): string {
  return new Intl.NumberFormat(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(micros / MICRO);
}

/**
 * Computes total amount from micro-unit values (TRX-026 formula).
 * Formula: floor(floor(qty × price / MICRO) × rate / MICRO) + fees
 *
 * All arguments and the return value are in micro-units.
 * Mirrors the backend integer arithmetic exactly — no decimal conversion involved.
 */
export function computeTotalMicro(
  qtyMicro: number,
  priceMicro: number,
  rateMicro: number,
  feesMicro: number,
): number {
  return Math.floor((Math.floor((qtyMicro * priceMicro) / MICRO) * rateMicro) / MICRO) + feesMicro;
}

/**
 * Computes sell total proceeds from micro-unit values (SEL-023 formula).
 * Formula: floor(floor(qty × price / MICRO) × rate / MICRO) − fees
 *
 * Fees are subtracted (not added) because they reduce the proceeds received.
 */
export function computeSellTotalMicro(
  qtyMicro: number,
  priceMicro: number,
  rateMicro: number,
  feesMicro: number,
): number {
  return Math.floor((Math.floor((qtyMicro * priceMicro) / MICRO) * rateMicro) / MICRO) - feesMicro;
}

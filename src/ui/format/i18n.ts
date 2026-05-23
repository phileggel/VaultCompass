/**
 * F27 layer-3 presenter output type: an i18n key plus optional interpolation vars.
 *
 * Returned by every per-BC `*MutationErrorToI18n` presenter; consumed by every
 * intermediate hook and component that needs to render a localised error string
 * via `t(message.key, message.vars)`. Lives under `ui/format/` per F28 because
 * it's a cross-feature primitive — not feature-owned, not bounded-context-scoped.
 */
export type I18nMessage = {
  key: string;
  vars?: Record<string, string | number>;
};

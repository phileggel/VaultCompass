# E2E Testability Rules

Defines what makes a component reliably driveable from the Tauri WebDriver E2E suite.
Read together with `docs/frontend-rules.md` and `docs/test_convention.md`.

⚠️ **AI AGENT MUST NEVER UPDATE THIS DOCUMENT**
**Rules numbering are indicative and not stable from version to version**

---

## E1 — Forms MUST have a stable `id` attribute

```tsx
<form id="price-modal-form" ...>
```

E2E tests locate forms by `id` — the most stable selector.
Naming convention: `{feature}-{action}-form` (e.g. `price-modal-form`, `edit-price-form`).

## E2 — Form fields MUST have a stable `id` attribute

```tsx
<input id="price-modal-date" ... />
<input id="price-modal-price" ... />
```

Convention: `{form-prefix}-{field}` (e.g. `price-modal-date`, `edit-price-price`).
The `id` MUST be forwarded to the underlying DOM `<input>` — never stop at the wrapper component.

## E3 — Submit buttons MUST use `type="submit"` and `form="{form-id}"`

```tsx
<Button
  type="submit"
  form="price-modal-form"
  disabled={isSubmitting || !isFormValid}
>
  Save
</Button>
```

E2E selector: `button[type="submit"][form="price-modal-form"]`.
Never rely on an `onClick`-only submit path — there is no stable selector for it.

## E4 — Navigation and action buttons MUST have a stable `aria-label` from i18n

```tsx
<IconButton aria-label={t("account_details.action_enter_price")} ... />
```

E2E selector: `button[aria-label="Enter price"]`.
Always use `t()` — never hardcode strings. Verify the exact English value in `en/common.json`.

## E5 — Error messages MUST have `role="alert"`

```tsx
<p role="alert" className="...">
  {t(error)}
</p>
```

E2E selector: `[role="alert"]` scoped to the form.
This is already required by accessibility rules — it is also what E2E tests assert.

## E6 — React controlled inputs require `setReactInputValue` in E2E tests

Standard `setValue()` from WebdriverIO does **not** reliably trigger React's synthetic
`onChange` in WebKitGTK. The DOM value is set but React state never updates, so
`isFormValid` stays `false` and the submit button stays disabled.

Use this helper in every E2E test file that sets input values:

```typescript
async function setReactInputValue(
  elementId: string,
  value: string,
): Promise<void> {
  await browser.execute(
    (id, val) => {
      const el = document.getElementById(id) as HTMLInputElement | null;
      if (!el) return;
      // Bypass React's value tracker via the native prototype setter, then
      // dispatch native events that React's delegation converts to synthetic onChange.
      const nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype,
        "value",
      )?.set;
      nativeSetter?.call(el, val);
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
    },
    elementId,
    value,
  );
}
```

After calling `setReactInputValue`, React re-renders synchronously within the same
event-loop tick. The next `waitForEnabled` poll will see the updated disabled state.

## E7 — Custom date pickers require locale-formatted input

`DateField` renders `<input type="text">` and displays dates in the active
i18n locale. `setReactInputValue` must receive the display format, not ISO,
**and the format must match the runtime locale** — otherwise
`useDateField.formatDateForStorage` parses the parts in the wrong positional
order (e.g. `01/04/2019` becomes Jan 4 under en-US instead of April 1) and the
form silently submits with the wrong date.

This project's E2E suite forces `LANG=en_US.UTF-8` in `wdio.conf.ts`
`beforeSession` so all aria-labels resolve to English. DateField therefore
runs in `en-US` → `MM/DD/YYYY`:

```typescript
// Convert ISO to the DateField display format. Must match the runtime locale.
// VaultCompass tests force en_US in wdio.conf.ts beforeSession.
function isoToDisplayDate(iso: string): string {
  const [year, month, day] = iso.split("-");
  return `${month}/${day}/${year}`; // "2020-01-15" → "01/15/2020" (en-US)
}

await setReactInputValue("price-modal-date", isoToDisplayDate("2020-01-15"));
```

`DateField.handleInputChange` then parses the display value back to ISO and calls
the parent `onChange` with the ISO date — React state updates correctly.

> If your project does NOT force a locale in `beforeSession`, the runtime
> locale follows `navigator.language` of the test runner host, which is usually
> `en-US` on GitHub-hosted runners but can vary. Either force a locale or
> pass an explicit `locale` prop to every `<DateField>` instance in tests.

## E8 — Tests MUST NOT call `browser.url()`

The Tauri WebView uses a custom protocol (`tauri://` or `http://tauri.localhost/`)
and is already loaded at the app's initial route when the session starts.
`browser.url()` breaks the WebView — navigate only through UI clicks.

## E9 — Tests MUST use deterministic, unique values per write operation

Use fixed past dates (not today's date) to avoid `DuplicateDate` errors from prior runs:

```typescript
// One constant per test that writes data — never share dates between seeding ops.
// Format must match the project's DateField locale (see E7).
const DATES = {
  record: isoToDisplayDate("2020-01-15"),
  update_original: isoToDisplayDate("2020-02-10"),
  delete: isoToDisplayDate("2020-03-05"),
} as const;
```

Today's date is pre-filled by default; always override it with a fixed past date.

## E10 — `waitForEnabled` / `waitForExist` MUST always specify `{ timeout: N }`

```typescript
await submitBtn.waitForEnabled({ timeout: 5000 });
await modal.waitForExist({ timeout: 8000, reverse: true });
```

Never rely on the WebdriverIO default timeout — always be explicit.

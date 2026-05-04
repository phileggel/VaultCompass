import assert from "node:assert";
import { browser } from "@wdio/globals";

export async function seedCategory(label: string): Promise<string> {
  const cat = (await browser.executeAsync((lbl: string, done: (r: unknown) => void) => {
    // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
    window.__TAURI_INTERNALS__
      .invoke("add_category", { label: lbl })
      .then(done)
      .catch((err: unknown) => done({ __error: String(err) }));
  }, label)) as { id: string };
  assert.ok(!("__error" in cat), `seedCategory failed: ${JSON.stringify(cat)}`);
  return cat.id;
}

export async function seedAccount(name: string, currency = "EUR"): Promise<string> {
  const acc = (await browser.executeAsync(
    (n: string, c: string, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("add_account", {
          dto: { name: n, currency: c, update_frequency: "ManualMonth" },
        })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    },
    name,
    currency,
  )) as { id: string };
  assert.ok(!("__error" in acc), `seedAccount failed: ${JSON.stringify(acc)}`);
  return acc.id;
}

export async function seedAsset(
  name: string,
  categoryId: string,
  options?: { reference?: string; assetClass?: string },
): Promise<string> {
  const reference = options?.reference ?? name.slice(0, 6).toUpperCase();
  const assetClass = options?.assetClass ?? "Stocks";
  const asset = (await browser.executeAsync(
    (n: string, ref: string, catId: string, cls: string, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("add_asset", {
          dto: {
            name: n,
            reference: ref,
            class: cls,
            category_id: catId,
            currency: "EUR",
            risk_level: 3,
          },
        })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    },
    name,
    reference,
    categoryId,
    assetClass,
  )) as { id: string };
  assert.ok(!("__error" in asset), `seedAsset failed: ${JSON.stringify(asset)}`);
  return asset.id;
}

export async function seedBuy(
  accountId: string,
  assetId: string,
  date: string,
  quantity: number,
): Promise<void> {
  const result = (await browser.executeAsync(
    (accId: string, astId: string, d: string, qty: number, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("buy_holding", {
          dto: {
            account_id: accId,
            asset_id: astId,
            date: d,
            quantity: qty,
            unit_price: 100_000_000,
            exchange_rate: 1_000_000,
            fees: 0,
            note: "",
          },
        })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    },
    accountId,
    assetId,
    date,
    quantity,
  )) as { id?: string; __error?: string };
  assert.ok(!("__error" in result), `seedBuy failed: ${JSON.stringify(result)}`);
}

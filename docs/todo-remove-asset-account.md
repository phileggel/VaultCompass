# TODO — Suppression d'`AssetAccount`

> Traiter séparément, avant ou en parallèle de l'implémentation TRX.
> Décision : ADR-002 (`docs/adr/002-replace-asset-account-with-holding.md`)

## Contexte

`AssetAccount` est du scaffolding inerte : le backend est complet (entity, repo, service, 3 commandes Tauri) mais aucun composant UI n'appelle ces commandes. Tout est tagué `TODO(R17)` ou commenté. La suppression est sans risque de régression.

## Backend (Rust)

- [ ] `migrations/` — Nouvelle migration : `DROP TABLE asset_accounts`, `CREATE TABLE holdings (id TEXT, account_id TEXT, asset_id TEXT, quantity INTEGER NOT NULL, average_price INTEGER NOT NULL)`
- [ ] `context/account/domain/asset_account.rs` — Supprimer le fichier
- [ ] `context/account/domain/mod.rs` — Retirer les exports `AssetAccount`, `AssetAccountRepository`
- [ ] `context/account/repository/asset_account.rs` — Supprimer le fichier
- [ ] `context/account/repository/mod.rs` — Retirer `SqliteAssetAccountRepository`
- [ ] `context/account/service.rs` — Retirer `asset_account_repo`, `get_holdings`, `upsert_holding`, `remove_holding` + tests associés
- [ ] `context/account/api.rs` — Retirer les 3 commandes (`get_account_holdings`, `upsert_account_holding`, `remove_account_holding`) + `UpsertHoldingDTO`
- [ ] `core/specta_builder.rs` — Retirer les 3 commandes du `collect_commands![]`
- [ ] `lib.rs` — Retirer `SqliteAssetAccountRepository` de l'init + `AccountService::new()`

## Frontend (TypeScript)

- [ ] `just generate-types` — Régénérer `bindings.ts` (retire `AssetAccount`, `UpsertHoldingDTO`, 3 commandes)
- [ ] `features/account_asset_details/` — Supprimer le dossier entier (placeholder inerte)
- [ ] `App.tsx` — Retirer import + route `"Account Details"` (onglet sidebar)
- [ ] `features/accounts/gateway.ts` — Retirer `getAccountHoldings`, `upsertAccountHolding`, `removeAccountHolding`
- [ ] `features/accounts/useAccounts.ts` — Retirer `getAccountHoldings`
- [ ] `features/accounts/add_account/useAddAccount.test.ts` — Retirer mock `getAccountHoldings`
- [ ] `features/accounts/edit_account_modal/useEditAccountModal.test.ts` — Retirer mock `getAccountHoldings`

## Note

L'onglet "Account Details" dans la sidebar disparaît avec la suppression d'`App.tsx`. Il sera recréé dans un meilleur état dans le cadre de l'implémentation TRX (vue des Holdings réels).

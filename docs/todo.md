# TODO

<!-- Ajouter les nouvelles dettes techniques et items de backlog ici. Format : ## (domaine) — Titre court -->

## (ai/agents) — i18n-checker : inclure les fichiers nouvellement ajoutés

L'agent utilise `git diff --name-only HEAD` + `git diff --name-only --cached` mais rate les nouveaux fichiers staged (`A ` dans `git status --porcelain`) quand ils n'ont pas encore de diff HEAD.
Ajouter `git status --porcelain | grep "^A " | awk '{print $2}'` à la liste des fichiers à analyser dans la définition de l'agent.

## (deps) — Mettre à jour specta vers rc.23

`tauri-specta rc.21` impose `specta = "=2.0.0-rc.22"` (version exacte). Attendre la sortie de `tauri-specta rc.22+` avant de passer à `specta rc.23` + `specta-typescript 0.0.10`.
État (2026-03-29) : `specta rc.23` disponible, `tauri-specta` toujours bloqué à `rc.21`.

## (frontend/ui) — Passer StatCard.tsx aux tokens M3

`StatCard` utilise `bg-emerald-100 text-emerald-700` / `bg-rose-100 text-rose-700` pour les badges positif/négatif.
Remplacer par `bg-m3-tertiary-container text-m3-on-tertiary-container` / `bg-m3-error-container text-m3-on-error-container`.

## (frontend/ui) — Supprimer les bordures structurelles dans ManagerLayout et ManagerHeader

`ManagerLayout` et `ManagerHeader` utilisent `border border-m3-outline/10` et `shadow-sm` (Tailwind brut).
Remplacer par `shadow-elevation-1` et différenciation tonale des surfaces.

## (frontend/ui) — Remplacer les couleurs Tailwind brutes dans AccountAssetDetailsView et Footer

`AccountAssetDetailsView.tsx` utilise `bg-white` et `border-gray-200` (hors tokens M3).
`Footer.tsx` utilise `bg-emerald-100 text-emerald-700` pour le badge de statut de connexion.
Remplacer par les tokens M3 sémantiques correspondants.

## (frontend) — Utiliser le logger centralisé dans les hooks de features

`useCategories.ts` utilise `console.error` directement au lieu du `logger` centralisé (`src/lib/logger.ts`).
(Note: `useAssets.ts` et `useAccounts.ts` sont désormais migrés.)
Remplacer les appels `console.error` par `logger.error` pour que les erreurs remontent au backend via `tracing`.

## (frontend/ui) — Créer le composant TextareaField

`AddTransactionModal` et `EditTransactionModal` utilisent une balise `<textarea>` brute pour le champ Note au lieu d'un composant partagé (violation F11/F12).
Créer `ui/components/field/TextareaField.tsx` (même interface que `TextField`, label + id + className + placeholder) et l'utiliser dans les deux modales.

## (frontend/transactions) — Feedback succès par snackbar (transactions)

`AddTransactionModal` et `EditTransactionModal` n'ont pas de retour visuel positif après soumission réussie (le modal se ferme silencieusement).
Brancher `showSnackbar(t("transaction.success_created"))` / `showSnackbar(t("transaction.success_updated"))` dans `doSubmit` une fois l'infrastructure toast en place.
Les clés i18n `transaction.success_created` et `transaction.success_updated` sont déjà définies dans `fr/common.json` et `en/common.json`.

## (frontend/assets) — Feedback succès par snackbar

Les mutations d'assets (création, modification, archivage) n'ont pas de feedback de succès visible.
Une fois la mécanique snackbar/toast en place (feature dédiée), brancher un appel `showSnackbar(t("asset.success_*"))` après chaque mutation dans `useAssets.ts`.

## (frontend/shell) — Implémenter la page Settings et câbler le bouton Settings de la Sidebar

Le bouton Settings du footer de `Sidebar.tsx` est câblé via `onSettingsClick?` mais `MainLayout` ne passe pas encore ce handler.
Créer la page Settings (feature `settings/`) et passer `onSettingsClick` depuis `MainLayout`.

## (frontend/transactions) — Buy button désactivé pour les assets archivés

Dans `AssetTable.tsx`, le bouton "Buy" (`ShoppingCart`) n'est pas désactivé pour les assets archivés (contrairement au bouton Edit qui est `disabled={asset.is_archived}`).
Ajouter `disabled={asset.is_archived}` avec un tooltip explicatif, ou laisser le modal gérer la confirmation (TRX-029 déjà en place).

## (frontend/transactions) — Déplacer la logique du Buy modal dans useAssetTable

La logique d'état pour le Buy modal (`isBuyModalOpen`, `buyPrefillAssetId`) est inline dans `AssetTable.tsx` au lieu d'être dans le hook `useAssetTable` (violation F10).
Déplacer dans `asset_table/useAssetTable.ts`.

## (frontend/transactions) — TRX-010: bouton "Add Transaction" dans la vue Account Details

L'entrée contextuelle depuis la vue "Account Details" (FAB ou bouton) est manquante (TRX-010).
La vue Account Details n'existe pas encore — `AccountAssetDetailsView` est un placeholder.
À implémenter quand la vue des positions par compte sera construite.

## (frontend/transactions) — TRX-035: dialog de confirmation de suppression d'une transaction

`deleteTransaction` est câblé dans `gateway.ts` et `useTransactions.ts`, mais aucun composant UI n'expose l'action de suppression avec une `ConfirmationDialog`.
À implémenter dans la vue transaction list (non encore créée).
Clés i18n `transaction.delete_confirm_title` et `transaction.delete_confirm_message` déjà définies.

## (spec/account) — Ajouter un champ `currency` à l'entité Account

La logique `showExchangeRate` dans `AddTransactionModal` et `EditTransactionModal` compare `selectedAsset.currency !== "EUR"` en dur.
Elle devrait comparer `selectedAsset.currency !== selectedAccount.currency`.
L'entité `Account` n'a pas de champ `currency` : il faut l'ajouter dans la spec account, puis dans le domaine, la migration SQL, les bindings, et les modales de transaction.
Impact TRX-021 et la règle de visibilité du champ Exchange Rate.

## (frontend/transactions) — TRX-038: implémenter l'affichage des positions (holdings)

`useTransactionStore.refreshHoldings()` est un stub: le backend a une table `holdings` (créée par la use case RecordTransaction), mais il n'existe pas de commande Tauri `getHoldings`.
Créer la commande `get_holdings(account_id) -> Vec<Holding>` dans `use_cases/record_transaction/api.rs` (ou `context/account/api.rs`) et l'utiliser dans `store.ts` pour afficher les positions par compte.

## (frontend/shell) — Renommer useSidebar.ts en navItems.ts

`useSidebar.ts` exporte uniquement des constantes et un type (pas de hook). Renommer en `navItems.ts` pour respecter la convention `use*` des hooks React.

## (frontend/shell) — Ajouter les logs de montage manquants (F13)

`Sidebar.tsx` et `DesignSystemPage.tsx` n'ont pas de `logger.info("[ComponentName] mounted")` dans leur `useEffect`. Ajouter conformément à la règle F13.

## ~~(frontend) — i18n des labels de navigation et du shell~~ ✅ résolu

Labels `NAV_ITEMS` et nom de l'app migrés vers i18n (`nav.*`). App renommée VaultCompass.

## From the two reviewers, the i18n findings (all pre-existing, not introduced by this migration):

  src/features/shell/useSidebar.ts — label values "Assets", "Accounts", "Categories", "About", "Design System" are
  hardcoded English strings rendered in three places: sidebar nav text, aria-label on nav buttons, and the <h1> page
  title via Header.tsx. They should be i18n keys resolved with t().

  src/features/shell/Header.tsx — resolveTitle() returns raw item.label strings, so the page title is hardcoded English.
   Fix is coupled to the useSidebar.ts fix above.

  src/features/shell/Sidebar.tsx — "Version: " prefix (expanded sidebar) is a hardcoded English string, e.g. should be
  t("shell.sidebar_version", { version: appVersion }).

  These were present before this task — the migration just exposed them because the files were touched. Want me to fix
  them as a follow-up?
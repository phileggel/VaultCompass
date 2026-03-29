# TODO

<!-- Ajouter les nouvelles dettes techniques et items de backlog ici. Format : ## (domaine) — Titre court -->

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

## (frontend) — Internationalisation (i18n) de tous les formulaires

Les labels et placeholders des formulaires `AssetForm` et `AccountForm` sont des chaînes anglaises codées en dur.
Ajouter `useTranslation` et les clés i18n correspondantes (en + fr).

## (frontend) — Implémenter les presenters et validateurs des features

Les fichiers `presenter.ts` et `validate*.ts` de toutes les features (`assets`, `accounts`, `categories`) sont des placeholders vides.
Y déplacer les transformations domaine → UI et la logique de validation des formulaires.

## (frontend) — Utiliser le logger centralisé dans les hooks de features

`useAssets.ts`, `useAccounts.ts` et `useCategories.ts` utilisent `console.error` directement au lieu du `logger` centralisé (`src/lib/logger.ts`).
Remplacer les appels `console.error` par `logger.error` pour que les erreurs remontent au backend via `tracing`.

## (frontend/shell) — Implémenter la page Settings et câbler le bouton Settings de la Sidebar

Le bouton Settings du footer de `Sidebar.tsx` est câblé via `onSettingsClick?` mais `MainLayout` ne passe pas encore ce handler.
Créer la page Settings (feature `settings/`) et passer `onSettingsClick` depuis `MainLayout`.

## (frontend/shell) — Renommer useSidebar.ts en navItems.ts

`useSidebar.ts` exporte uniquement des constantes et un type (pas de hook). Renommer en `navItems.ts` pour respecter la convention `use*` des hooks React.

## (frontend/shell) — Ajouter les logs de montage manquants (F13)

`Sidebar.tsx` et `DesignSystemPage.tsx` n'ont pas de `logger.info("[ComponentName] mounted")` dans leur `useEffect`. Ajouter conformément à la règle F13.

## (frontend) — i18n des labels de navigation et du shell

Les labels de `NAV_ITEMS` ("Assets", "Accounts", etc.), le nom de l'app ("Vault M3"), et les tooltips du menu hamburger ("Collapse menu" / "Expand menu") sont des chaînes anglaises codées en dur dans `Sidebar.tsx`.
À traiter dans le sprint i18n général — nécessite de séparer les clés de routage (`navKey`) des labels traduits pour éviter de casser la navigation.

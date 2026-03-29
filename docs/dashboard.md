# Règles métier — Dashboard de performance d'un compte

## Contexte

L'utilisateur souhaite visualiser la performance d'un compte précis sur le temps : évolution de la valeur totale période par période, répartition par classe d'actif, gain/perte par rapport au coût historique, et indices base 100 de performance pure. La granularité temporelle (mensuelle ou annuelle) est déterminée par l'`UpdateFrequency` du compte. La feature s'appuie sur les entités existantes — `Account`, `Asset`, `AssetCategory`, `AssetPrice` — et sur deux entités à créer dans des specs dédiées : `Operation` (historique des achats/ventes, spec `operations.md`) et `ExchangeRate` (taux de change manuels, spec `account-currency.md`). La commande de performance est un use case cross-contextuel dans `use_cases/`.

> **Dépendances** : cette spec ne peut être implémentée qu'après la spec `operations.md` (historique des opérations) et la spec `account-currency.md` (devise par compte + taux de change).

---

## Règles métier

**R1 — Valeur d'un actif sur une période (backend)** : La valeur d'un actif détenu dans un compte pour une période donnée est `quantité_reconstituée × dernier AssetPrice.price`, où la quantité est reconstituée à la fin de la période à partir de l'historique des opérations (spec `operations.md`), et le prix est le dernier `AssetPrice.price` dont la date est antérieure ou égale au dernier jour de la période. Le dernier jour d'une période mensuelle est le dernier jour calendaire du mois (ex. 31 jan, 28/29 fév) ; pour une période annuelle, c'est le 31 décembre. Si aucun prix n'est disponible avant ce seuil, l'actif est exclu du calcul de cette période (jamais de valeur zéro implicite). Les valeurs sont converties dans la devise de référence du compte (`Account.currency`) en utilisant les taux de change saisis manuellement (spec `account-currency.md`).

**R2 — Valeur totale d'un compte sur une période (backend)** : La valeur totale du compte pour une période est la somme des valeurs de tous ses holdings calculées selon R1.

**R3 — Granularité temporelle déterminée par UpdateFrequency (backend + frontend)** : La granularité est déterminée par l'`UpdateFrequency` du compte : `ManualMonth` ou `Automatic` → périodes mensuelles ; `ManualYear` → périodes annuelles. Les fréquences `ManualDay` et `ManualWeek` utilisent la granularité mensuelle par défaut.

**R4 — Plage temporelle complète (frontend)** : Le dashboard affiche toujours l'intégralité de l'historique disponible pour le compte, sans sélecteur de plage ni troncature temporelle.

**R5 — Progression par période (backend)** : La progression d'une période est `valeur_n − valeur_{n−1}` (absolue) et `(valeur_n − valeur_{n−1}) / valeur_{n−1}` (relative). Si la période précédente n'a pas de valeur calculable, les deux progressions sont `null`.

**R6 — Base 100 YTD (backend)** : La référence YTD est la valeur totale du compte à la fin de la dernière période de l'année civile précédente (ex. fin décembre N−1), posée à 100 (non visible dans le tableau). Cette valeur de référence exige que **tous** les actifs du compte aient un prix disponible à cette date ; si au moins un actif manque de prix, la référence est absente et la base 100 YTD affiche "—" pour toute l'année. Formule : `base_n = base_{n−1} × (1 + progression_n)`. Le premier mois affiché peut être ≠ 100. Si aucune valeur complète n'est disponible en fin d'année précédente, la référence est le dernier mois où tous les actifs ont un prix, avant le premier mois de l'année.

**R7 — Base 100 historique (backend)** : La référence historique est la valeur du compte à la fin de la période précédant le premier mois disponible dans l'historique complet, posée à 100 (non visible). Même formule que R6. Si le premier mois disponible n'a pas de précédent, ce premier mois est lui-même la référence (= 100) et s'affiche comme 100. Ce calcul ne tient pas compte des apports de capital entre périodes (simplification acceptée).

**R8 — Valeur par catégorie dans le tableau (backend)** : Pour chaque période, le backend calcule la valeur de chaque catégorie d'actifs (`AssetCategory`) présente dans le compte selon R1. Ces valeurs sont retournées dans `CategoryValue { category_id, category_name, value }`. Une catégorie sans aucun prix disponible pour une période retourne `null`.

**R9 — Colonnes catégories dans la vue tableau (frontend)** : Les colonnes catégories sont dynamiques (une colonne par catégorie distincte présente dans les résultats). Le tableau est scrollable horizontalement quand le nombre de colonnes dépasse la largeur visible.

**R10 — Performance absolue et relative du compte (backend)** : Calculée à partir de l'historique des opérations (spec `operations.md`) en méthode VWAP. Pour chaque actif : `gain = (prix_actuel − prix_moyen_VWAP) × quantité_actuelle`. La performance totale du compte est la somme des gains de tous les actifs, convertis dans la devise de référence du compte. `relative_gain_pct = absolute_gain / coût_total_VWAP × 100`. Si le coût total est nul, `relative_gain_pct` est `null`.

**R11 — Ordre d'affichage des périodes (frontend)** : Les périodes sont affichées en ordre chronologique ascendant (la plus ancienne en premier, la plus récente en bas).

**R12 — Période sans données (frontend)** : Une période sans aucun prix disponible pour aucun holding affiche "—" dans toutes ses cellules numériques — jamais de zéro, pour ne pas fausser les progressions et les bases.

**R13 — Vue graphique (frontend)** : La vue graphique affiche un graphique en barres verticales avec une barre par période, dont la hauteur représente la valeur totale du compte (R2). Les périodes sans données (R12) sont représentées par une barre absente ou un indicateur visuel distinct. Trois cartes indicateurs sont affichées au-dessus du graphique : valeur totale actuelle, gain absolu total, gain relatif total (en %).

**R14 — Commande backend dédiée dans use_cases/ (backend)** : La commande Tauri `get_account_performance(account_id)` est implémentée dans `use_cases/` car elle requiert des données de plusieurs contextes : `AccountRepository` + `OperationRepository` (context/account), `AssetRepository` + `AssetCategoryRepository` + `PriceRepository` (context/asset), et `ExchangeRateRepository` (context/account-currency), injectés comme dépendances. Elle retourne `AccountPerformanceResult { periods: Vec<AccountPeriod>, performance: AccountPerformance }` avec `AccountPeriod { period_label, total_value, progression_abs, progression_pct, base100_ytd, base100_all, category_values: Vec<CategoryValue { category_id, category_name, value }> }`.

**R15 — Compte inexistant (backend)** : Si l'`account_id` fourni n'existe pas, la commande retourne une erreur explicite (non un résultat vide).

**R16 — Accès depuis la vue compte (frontend)** : Le dashboard est accessible depuis `AccountAssetDetailsView` via un onglet "Performance". Il n'est pas un item du drawer de navigation.

**R17 — Pas de benchmark externe dans cette version (frontend)** : La comparaison avec un indice externe (CAC40, etc.) est hors périmètre de cette version.

---

## Workflow

```
[Utilisateur navigue vers un compte]
  → Clique sur l'onglet "Performance"
          │
          ▼
[Appel get_account_performance(account_id)]
  → Tout l'historique retourné
  → Granularité lue depuis Account.update_frequency
          │
          ├─→ Vue graphique : barres de valeur totale par période
          │                   + cartes : valeur actuelle, gain €, gain %
          └─→ Vue tableau   : une ligne par période
                              Période | Prog (€) | Prog (%) | Valeur totale
                              | Cat.1 | Cat.2 | … | Base 100 YTD | Base 100 Hist.
```

---

## Maquette UX

### Point d'entrée

Depuis `AccountAssetDetailsView` — onglet "Performance" en haut de la vue.

### Composant principal

Panel intégré dans la vue compte (pas de modal). Deux sous-vues accessibles par onglets :

1. **Vue graphique** — graphique en barres (valeur totale par période) + 3 cartes indicateurs
2. **Vue tableau** — grille avec une ligne par période, scroll horizontal

### Vue tableau — colonnes et exemple

| Période  | Prog (€) | Prog (%) | Valeur totale | [Cat. A] | [Cat. B] | …   | Base 100 YTD | Base 100 Hist. |
| -------- | -------- | -------- | ------------- | -------- | -------- | --- | ------------ | -------------- |
| Jan 2025 | +370 €   | +3,1 %   | 12 450 €      | 5 200 €  | 7 250 €  | …   | 103,1        | 134,2          |
| Fév 2025 | +320 €   | +2,6 %   | 12 770 €      | 5 400 €  | 7 370 €  | …   | 105,8        | 137,7          |
| Mar 2025 | —        | —        | —             | —        | —        | …   | —            | —              |

> La base 100 YTD de janvier est > 100 car la référence est la fin décembre de l'année précédente.

### États

- **Vide** : "Aucune donnée de prix disponible. Commencez par saisir les prix de vos actifs."
- **Chargement** : Squelettes sur le graphique et le tableau
- **Données partielles** : Cellules sans données affichées avec "—" + tooltip "Prix non saisi pour cette période"
- **Erreur** : Message d'erreur avec bouton Réessayer

### Flux utilisateur

1. L'utilisateur navigue vers un compte dans `AccountAssetDetailsView`.
2. Il clique sur l'onglet "Performance".
3. La vue graphique s'affiche par défaut avec les barres de valeur et les 3 cartes.
4. Il bascule sur la vue tableau pour voir les chiffres détaillés ligne par ligne.
5. En survolant une cellule "Base 100", un tooltip indique la période de référence (ex. "Référence : Déc 2024 = 100").

---

## Dépendances

Cette spec ne peut être implémentée qu'après :

- **`docs/operations.md`** — historique des opérations (achat/vente), calcul VWAP, reconstitution du portefeuille à une date passée. Remplace la saisie directe de `AssetAccount.average_price` et `AssetAccount.quantity`.
- **`docs/account-currency.md`** — devise de référence par compte (`Account.currency`), taux de change manuels (`ExchangeRate`), conversion des valeurs dans la devise du compte.

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

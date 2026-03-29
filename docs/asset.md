# Règles métier — Gestion des assets et catégories

## Contexte

Un asset représente un instrument financier ou une ressource détenue par l'utilisateur : action, ETF, obligation, bien immobilier, cryptomonnaie, etc. Chaque asset appartient à une catégorie utilisateur (ex. « Actions Europe », « Immo ») et porte une devise ISO 4217 qui est la devise de cotation du titre. La gestion des assets et des catégories est le socle du reste de l'application : un compte (`Account`) regroupe des assets via des opérations (`Operation`) ; le dashboard de performance s'appuie sur les assets pour calculer la valeur d'un portefeuille.

Cette spec couvre la création, la modification et la suppression des entités `Asset` et `AssetCategory`, côtés backend et frontend. Les prix des assets (`AssetPrice`) sont traités dans la spec `docs/operation.md`.

---

## Définition des champs d'un asset

| Champ        | Signification                                                                                                                                                                                                                                                      |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `name`       | Nom lisible de l'instrument (ex. « Apple Inc. », « SCPI Pierval »)                                                                                                                                                                                                 |
| `class`      | Type d'actif financier parmi les valeurs fixes de `AssetClass` (Actions, ETF, Immobilier, Cash, Obligations, Fonds, Crypto)                                                                                                                                        |
| `category`   | Regroupement libre défini par l'utilisateur (ex. « Actions US », « Immo Europe »). Sert à agréger les valeurs dans le dashboard de performance. Ce n'est pas une taxonomie fixe : l'utilisateur crée ses propres catégories.                                       |
| `currency`   | Devise de **cotation** du titre (ISO 4217 : USD, EUR, BTC…). C'est la devise dans laquelle le prix de l'asset est exprimé — distincte de la devise de référence du compte. Ex. : une action Apple cotée en USD dans un compte dont la devise de référence est EUR. |
| `risk_level` | Score de risque subjectif de 1 (risque faible) à 5 (risque élevé). Le frontend suggère une valeur par défaut selon la `class` choisie (voir R2), modifiable manuellement par l'utilisateur.                                                                        |
| `reference`  | Identifiant externe du titre : ticker boursier (ex. `AAPL`), code ISIN (ex. `FR0000131104`), ou identifiant interne auto-généré (ex. `INT-STOCKS-A3F2`) pour les actifs non cotés sur un marché officiel.                                                          |

---

## Règles métier

### Asset — Backend

**R1 — Champs requis d'un asset (backend)** : Un asset est valide si et seulement si : `name` est non vide, `class` est une valeur de `AssetClass`, `currency` est un code ISO 4217 valide, `risk_level` est un entier entre 1 et 5 inclus. La `category` est obligatoire ; si l'utilisateur n'en choisit pas, la catégorie par défaut `default-uncategorized` est assignée automatiquement.

**R2 — Classes d'assets (backend)** : La classification (`AssetClass`) est une liste fixe pré-seedée, non personnalisable par l'utilisateur :

| Classe         | `default_risk` |
| -------------- | -------------- |
| `Cash`         | 1              |
| `Bonds`        | 2              |
| `MutualFunds`  | 3              |
| `ETF`          | 3              |
| `Stocks`       | 4              |
| `RealEstate`   | 4              |
| `DigitalAsset` | 5              |

La valeur par défaut est `Cash`.

**R3 — Référence normalisée (backend)** : Le champ `reference` est optionnel à la saisie. S'il est fourni, il est normalisé : espaces de début et fin supprimés, converti en majuscules (les espaces internes sont conservés). S'il est absent ou vide, le backend génère automatiquement une référence interne de la forme `INT-{CLASS}-{SHORT_ID}` (ex. `INT-STOCKS-A3F2`).

**R4 — Mise à jour d'un asset (backend)** : Tous les champs sont modifiables après création. Les mêmes validations que R1–R3 s'appliquent. Si la référence est remise à vide lors d'une modification, une nouvelle référence interne est générée.

**R5 — Suppression d'un asset en cascade (backend)** : La suppression d'un asset entraîne la suppression en cascade de toutes ses données dépendantes : opérations (`Operation`), prix (`AssetPrice`) et holdings dans tous les comptes (`AssetAccount`). Cette action est irréversible.

### Asset — Frontend

**R6 — Tableau des assets — colonnes (frontend)** : Le tableau affiche les colonnes suivantes, dans cet ordre, trié par défaut par Nom ascendant :

| Colonne   | Contenu                                                | Triable |
| --------- | ------------------------------------------------------ | ------- |
| Nom       | `asset.name` — texte principal                         | Oui     |
| Référence | `asset.reference` — police monospace, `—` si absent    | Oui     |
| Classe    | `asset.class` — chip outline                           | Oui     |
| Catégorie | `asset.category.name`                                  | Oui     |
| CCY       | `asset.currency` — centré, gras, uppercase             | Oui     |
| Risque    | `asset.risk_level` — badge circulaire coloré (voir R9) | Oui     |
| Actions   | Boutons Éditer + Supprimer                             | Non     |

Un en-tête de page affiche le titre « Assets », le nombre total d'assets et un champ de recherche fuzzy filtrant sur nom, référence, classe et catégorie.

**R7 — Création d'un asset via FAB (frontend)** : Un bouton FAB flottant en bas à droite ouvre une modal de création. Le formulaire contient : Nom (requis), Référence (optionnel), Devise ISO (requis), Catégorie (select), Classe (select), Niveau de risque (sélecteur visuel 1–5, voir R9). La soumission est bloquée si le nom ou la devise est absent.

**R8 — Avertissement de doublon de référence (frontend)** : Lors de la création ou de la modification d'un asset, si la référence saisie correspond (casse ignorée) à la référence d'un asset existant, quelle que soit la classe, un avertissement non bloquant est affiché dans le formulaire. L'utilisateur peut ignorer l'avertissement et confirmer.

**R9 — Niveau de risque — suggestion et affichage (frontend)** : Lorsque l'utilisateur sélectionne une classe dans le formulaire, le champ `risk_level` est automatiquement pré-rempli avec le `default_risk` de cette classe (R2), modifiable ensuite manuellement. Le sélecteur est un groupe de 5 boutons radio visuels (1–5), le bouton actif affiché en couleur primaire. Dans le tableau : badge circulaire coloré — vert pour 1–2, orange pour 3, rouge pour 4–5.

**R10 — Modification d'un asset (frontend)** : Chaque ligne du tableau expose un bouton icône Éditer. Il ouvre une modal présentant le même formulaire que la création, pré-rempli avec les valeurs actuelles de l'asset. Après sauvegarde, la modal se ferme et le tableau se rafraîchit.

**R11 — Suppression d'un asset (frontend)** : Chaque ligne expose un bouton icône Supprimer (destructif). Il ouvre une dialog de confirmation listant les comptes impactés par la suppression (comptes dans lesquels l'asset est détenu via `AssetAccount`) et mentionnant que l'opération est irréversible. La confirmation déclenche la suppression en cascade (R5).

**R12 — États d'erreur (frontend)** : Tout échec d'appel backend (création, modification, suppression) affiche un message d'erreur inline dans la modal ou la dialog active. Le tableau expose un état d'erreur avec bouton Réessayer si le chargement initial échoue.

### Catégorie

Les règles CRUD de `AssetCategory` sont définies dans la spec dédiée `docs/category.md`.

---

## Workflow

```
[Utilisateur ouvre « Assets »]
  → Tableau d'assets (tri défaut : Nom asc) + FAB
          │
          ├─ [Recherche] → Filtre fuzzy temps réel
          ├─ [Clic en-tête] → Tri ascendant/descendant
          │
          ├─ [FAB] → Modal création
          │            → Sélection classe → risk_level pré-rempli (R9)
          │            → Avertissement doublon si référence existante (R8)
          │            → Soumission → asset créé → modal fermée → tableau rafraîchi
          │
          ├─ [Éditer] → Modal édition pré-remplie → Modification → tableau rafraîchi
          │
          └─ [Supprimer] → Dialog (liste comptes impactés + mention irréversibilité)
                            → Confirmation → Suppression cascade
```

---

## Maquette UX

### Point d'entrée

**Assets** — item du drawer de navigation principal.

### Composant principal

Page avec tableau pleine largeur, trié par défaut par Nom ascendant. FAB flottant en bas à droite. Boutons Éditer (icône primaire) et Supprimer (icône destructif) sur chaque ligne.

### États

- **Vide** : « Aucun asset trouvé. »
- **Chargement** : Indicateur de chargement dans le tableau
- **Erreur de chargement** : Message d'erreur + bouton Réessayer
- **Avertissement doublon** : Bannière inline dans la modal, non bloquante (R8)
- **Confirmation suppression** : Dialog listant les comptes impactés, mention irréversibilité
- **Erreur backend** : Message d'erreur inline dans la modal ou dialog active

### Flux utilisateur — création d'un asset

1. L'utilisateur clique sur le FAB → modal de création s'ouvre.
2. Il sélectionne une classe → `risk_level` pré-rempli automatiquement.
3. Il remplit les autres champs. Si la référence existe déjà → avertissement non bloquant.
4. Il soumet → asset créé → modal fermée → tableau rafraîchi.

### Flux utilisateur — suppression d'un asset

1. L'utilisateur clique sur Supprimer → dialog listant les comptes impactés.
2. Il confirme → suppression en cascade (opérations, prix, holdings).

---

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

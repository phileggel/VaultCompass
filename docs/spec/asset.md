# Règles métier — Gestion des assets

## Contexte

Un asset représente un instrument financier ou une ressource détenue par l'utilisateur : action, ETF, obligation, bien immobilier, cryptomonnaie, etc. Chaque asset appartient à une catégorie utilisateur (ex. « Actions Europe », « Immo ») et porte une devise ISO 4217 qui est la devise de cotation du titre. La gestion des assets est le socle du reste de l'application : un compte (`Account`) regroupe des assets via des opérations (`Operation`) ; le dashboard de performance s'appuie sur les assets pour calculer la valeur d'un portefeuille.

Cette spec couvre la création, la modification et l'archivage de l'entité `Asset`, côtés backend et frontend. Les règles CRUD de `AssetCategory` sont dans `docs/category.md`. Les prix des assets (`AssetPrice`) sont traités dans `docs/operation.md`.

> Note : la `reference` seule ne suffit pas à identifier un instrument de manière unique à l'échelle du pricing (un même ticker peut exister sur plusieurs places avec des devises différentes). La clé de déduplication pour le pricing sera définie dans la spec dédiée.

---

## Définition des champs d'un asset

### `name`

Nom lisible de l'instrument (ex. « Apple Inc. », « SCPI Pierval »).

### `class`

Type d'actif financier parmi les valeurs fixes de `AssetClass` (voir R3). Non personnalisable par l'utilisateur.

### `category`

Regroupement libre défini par l'utilisateur (ex. « Actions US », « Immo Europe »). Sert à agréger les valeurs dans le dashboard de performance. Ce n'est pas une taxonomie fixe : l'utilisateur crée ses propres catégories.

### `currency`

Devise de **cotation** du titre (ISO 4217 : USD, EUR, BTC…). C'est la devise dans laquelle le prix de l'asset est exprimé — distincte de la devise de référence du compte. Ex. : une action Apple cotée en USD dans un compte dont la devise de référence est EUR.

### `risk_level`

Score de risque subjectif de 1 (risque faible) à 5 (risque élevé). Le frontend suggère une valeur par défaut selon la `class` choisie (voir R3), modifiable manuellement.

### `reference`

Identifiant du titre : ticker boursier (ex. `AAPL`), code ISIN (ex. `FR0000131104`), ou identifiant libre saisi par l'utilisateur (ex. `APPART-PARIS-15`) pour les actifs non cotés. Obligatoire.

### `is_archived`

Indique si l'asset est archivé (retiré des listes actives). Un asset archivé conserve toutes ses données historiques mais ne peut plus être modifié ni recevoir de nouveaux prix.

---

## Règles métier

### Asset — Backend

**R1 — Validation des champs (backend)** : Un asset est valide si et seulement si : `name` est non vide, `reference` est non vide, `category` est renseignée, `class` est une valeur de `AssetClass`, `currency` est un code ISO 4217 valide, et `risk_level` est un entier entre 1 et 5 inclus. Toute violation est rejetée par le backend avec une erreur explicite.

**R3 — Classes d'assets et risque par défaut (backend)** : La classification (`AssetClass`) est une liste fixe pré-seedée, non personnalisable par l'utilisateur :

| Classe         | `default_risk` |
| -------------- | -------------- |
| `Cash`         | 1              |
| `Bonds`        | 2              |
| `MutualFunds`  | 3              |
| `ETF`          | 3              |
| `Stocks`       | 4              |
| `RealEstate`   | 2              |
| `DigitalAsset` | 5              |

La valeur par défaut est `Cash`.

**R4 — Normalisation de la référence (backend)** : La référence est normalisée à la réception : espaces de début et fin supprimés, convertie en majuscules (espaces internes conservés).

**R5 — Mise à jour d'un asset (backend)** : Tous les champs d'un asset sont modifiables après création. Les règles de validation (R1) et de normalisation de la référence (R4) s'appliquent à la modification comme à la création.

**R6 — Archivage d'un asset (backend)** : Archiver un asset positionne `is_archived = true`. L'asset disparaît des listes actives mais toutes ses données associées sont conservées (opérations, prix, holdings). Un asset archivé ne peut plus recevoir de nouveaux prix ni être modifié.

**R18 — Désarchivage d'un asset (backend)** : Désarchiver un asset positionne `is_archived = false`. L'asset redevient actif, réapparaît dans les listes actives et peut à nouveau être modifié et recevoir de nouveaux prix.

### Asset — Frontend

**R2 — Pré-sélection de la catégorie (frontend)** : Le formulaire pré-sélectionne `default-uncategorized` si aucune catégorie n'est choisie par l'utilisateur, garantissant que le champ est toujours renseigné à la soumission.

**R7 — Tableau des assets (frontend)** : Le tableau affiche les colonnes suivantes, dans cet ordre, trié par défaut par Nom ascendant :

| Colonne   | Contenu                                    | Triable |
| --------- | ------------------------------------------ | ------- |
| Nom       | `asset.name`                               | Oui     |
| Référence | `asset.reference`                          | Oui     |
| Classe    | `asset.class`                              | Oui     |
| Catégorie | `asset.category.name`                      | Oui     |
| CCY       | `asset.currency`                           | Oui     |
| Risque    | `asset.risk_level` — badge de risque (R11) | Oui     |
| Statut    | Badge « Archivé » si `is_archived = true`  | Non     |
| Actions   | Voir R13, R19, R20                         | Non     |

Le tableau affiche uniquement les assets actifs (`is_archived = false`) par défaut. Un en-tête de page affiche le titre « Assets » et le nombre total d'assets actifs.

**R16 — Recherche fuzzy (frontend)** : Un champ de recherche dans l'en-tête filtre la liste en temps réel sur nom, référence, classe et catégorie. La recherche s'applique uniquement aux assets actuellement affichés : assets actifs seuls si le toggle R19 est désactivé, actifs et archivés si le toggle est activé. Si aucun résultat ne correspond, le tableau affiche « Aucun résultat pour cette recherche. »

**R17 — Tri des colonnes (frontend)** : Un clic sur un en-tête de colonne triable trie la liste par cette colonne en ordre ascendant. Un second clic bascule en ordre descendant.

**R8 — Création via FAB (frontend)** : Un bouton FAB flottant en bas à droite ouvre une modal de création. Le formulaire contient : Nom (requis), Référence (requis), Devise ISO (requis), Catégorie (select, pré-sélectionnée sur `default-uncategorized`, voir R2), Classe (select, pré-sélectionnée sur `Cash`), Niveau de risque (sélecteur 1–5, pré-rempli selon la classe, voir R10). La soumission est bloquée si le nom, la référence ou la devise est absent.

**R9 — Avertissement de doublon de référence (frontend)** : Lors de la création ou de la modification d'un asset, si la référence saisie correspond (casse ignorée) à la référence d'un asset existant — actif ou archivé — quelle que soit la classe, un avertissement non bloquant est affiché dans le formulaire. L'utilisateur peut ignorer l'avertissement et confirmer. L'avertissement est intentionnellement non bloquant : un même identifiant peut désigner des instruments légitimement distincts selon leur devise de cotation ou leur place de marché. Les assets archivés sont inclus dans la vérification pour éviter les doublons silencieux en cas de désarchivage ultérieur.

**R10 — Suggestion du niveau de risque à la création (frontend)** : À la création uniquement, lorsque l'utilisateur sélectionne une classe, le champ `risk_level` est automatiquement pré-rempli avec le `default_risk` de cette classe (R3), modifiable ensuite manuellement.

**R11 — Badge de risque dans le tableau (frontend)** : Le niveau de risque est affiché dans le tableau sous forme de badge coloré, une couleur par niveau : vert clair (1), vert (2), orange (3), rouge clair (4), rouge (5).

**R12 — Modification d'un asset (frontend)** : Le bouton Éditer ouvre une modal présentant le même formulaire que la création, pré-rempli avec les valeurs actuelles de l'asset. Les mêmes règles de validation s'appliquent (R8) : la soumission est bloquée si un champ requis est absent. Le `risk_level` existant est affiché tel quel et n'est jamais remplacé automatiquement lors d'un changement de classe — la suggestion automatique (R10) ne s'applique pas en mode édition. Après sauvegarde, la modal se ferme et le tableau se rafraîchit.

**R13 — Archivage d'un asset (frontend)** : Le bouton Archiver ouvre une dialog de confirmation indiquant que l'asset sera retiré des listes actives et ne pourra plus recevoir de nouveaux prix, mais que toutes ses données historiques sont conservées. La confirmation déclenche l'archivage (R6).

**R14 — Erreurs backend (frontend)** : La modal reste ouverte pendant l'appel backend et ne se ferme qu'en cas de succès. Tout échec affiche un message d'erreur inline dans la modal ou la dialog active.

**R15 — État d'erreur de chargement (frontend)** : Si le chargement initial de la liste échoue, le tableau affiche un message d'erreur avec un bouton Réessayer.

**R19 — Toggle assets archivés (frontend)** : L'en-tête expose un toggle « Afficher les archivés ». Lorsqu'il est activé, les assets archivés apparaissent dans le tableau avec un style visuel atténué sur l'ensemble de la ligne (pas uniquement le badge) permettant de distinguer immédiatement les assets actifs des archivés. Le bouton Archiver est remplacé par un bouton Désarchiver sur les lignes archivées ; le bouton Éditer est désactivé.

**R20 — Désarchivage depuis le tableau (frontend)** : Le bouton Désarchiver (visible uniquement sur les lignes archivées, lorsque le toggle R19 est actif) ouvre une dialog de confirmation. La confirmation déclenche le désarchivage (R18) et l'asset réapparaît dans la liste active.

---

## Workflow

```
[Utilisateur ouvre « Assets »]
  → Tableau d'assets (tri défaut : Nom asc) + FAB
          │
          ├─ [Recherche] → Filtre fuzzy temps réel (R16)
          ├─ [Clic en-tête] → Tri ascendant/descendant (R17)
          │
          ├─ [FAB] → Modal création
          │            → Sélection classe → risk_level pré-rempli (R10)
          │            → Avertissement doublon si référence existante (R9)
          │            → Soumission → asset créé → modal fermée → tableau rafraîchi
          │
          ├─ [Éditer] → Modal édition pré-remplie → Modification → tableau rafraîchi
          │
          ├─ [Archiver] → Dialog (asset retiré des listes actives, données conservées)
          │               → Confirmation → Archivage (R6)
          │
          └─ [Toggle archivés] → Affiche les assets archivés avec bouton Désarchiver (R19)
                                  → [Désarchiver] → Dialog confirmation → Désarchivage (R18/R20)
```

---

## Maquette UX

### Point d'entrée

**Assets** — item du drawer de navigation principal.

### Composant principal

Page avec tableau pleine largeur, trié par défaut par Nom ascendant. FAB flottant en bas à droite. Boutons Éditer (icône primaire) et Archiver (icône d'archivage) sur chaque ligne.

### États

- **Vide** : « Aucun asset. Créez votre premier asset avec le bouton +. »
- **Chargement** : Indicateur de chargement dans le tableau
- **Erreur de chargement** : Message d'erreur + bouton Réessayer (R15)
- **Avertissement doublon** : Bannière inline dans le formulaire, non bloquante (R9)
- **Confirmation archivage** : Dialog expliquant que l'asset sera retiré des listes actives et que les données historiques sont conservées (R13)
- **Assets archivés visibles** : Lignes distinctes visuellement, bouton Désarchiver à la place d'Archiver, bouton Éditer désactivé (R19)
- **Confirmation désarchivage** : Dialog de confirmation avant réactivation (R20)
- **Erreur backend** : Message d'erreur inline dans la modal ou dialog active (R14)

### Flux utilisateur — création d'un asset

1. L'utilisateur clique sur le FAB → modal de création s'ouvre.
2. Il sélectionne une classe → `risk_level` pré-rempli automatiquement (R10).
3. Il remplit les autres champs. Si la référence existe déjà → avertissement non bloquant (R9).
4. Il soumet → asset créé → modal fermée → tableau rafraîchi.

### Flux utilisateur — archivage d'un asset

1. L'utilisateur clique sur Archiver → dialog expliquant que l'asset sera retiré des listes actives et que les données sont conservées.
2. Il confirme → archivage (R6).

---

## Features futures

### Suppression définitive d'un asset

Permettre la suppression physique (hard delete) d'un asset archivé, uniquement si aucune opération n'y est rattachée. Si des opérations existent, la suppression définitive est bloquée — l'archivage reste la seule option. Cette feature n'est pas dans le périmètre de l'implémentation actuelle.

### Historique des opérations d'un asset

Afficher, depuis la page Assets, la liste des opérations rattachées à un asset donné. Point d'entrée envisagé : action contextuelle sur la ligne de l'asset dans le tableau, ouvrant un panneau ou une sous-page dédiée. Cette feature sera traitée dans la spec `operation`.

### Feedback de succès par snackbar

Afficher une notification snackbar après les opérations de mutation (création, modification, archivage, désarchivage), en remplacement du simple retour visuel « modal fermée ». Nécessite la mise en place préalable d'un composant snackbar dans le design system.

---

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

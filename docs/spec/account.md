# Règles métier — Gestion des comptes

## Contexte

Un `Account` représente un compte financier détenu par l'utilisateur (ex. compte-titres, PEA, assurance-vie, compte courant). Chaque compte peut regrouper des positions sur des assets (`Holding`) créées via des transactions.

L'application distingue deux vues liées aux comptes :

1. **Vue CRUD** (cette spec) — permet de créer, renommer et supprimer des comptes.
2. **Vue gestion** (spec future) — permet de consulter la valeur d'un compte, d'acheter ou revendre des assets, et de visualiser les performances. Cette vue s'appuie sur les transactions (`docs/transaction.md`) pour construire et mettre à jour les `Holding`.

---

## Définition des entités

### Account

Un compte financier appartenant à l'utilisateur.

| Champ              | Signification métier                                                                                                    |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `name`             | Nom lisible du compte défini par l'utilisateur (ex. « PEA Boursorama », « Livret A »). Unique parmi les comptes actifs. |
| `update_frequency` | Fréquence à laquelle l'utilisateur prévoit de mettre à jour les données du compte. Purement informative aujourd'hui.    |

---

## Règles métier

### Backend

**R1 — Normalisation du nom (backend)** : Le `name` est normalisé à la réception avant toute validation ou stockage : les espaces de début et fin sont supprimés.

**R2 — Validation Account (backend)** : Un `Account` est valide si et seulement si son `name` (après normalisation R1) est non vide, et son `update_frequency` est une valeur parmi les cinq valeurs fixes (`Automatic`, `ManualDay`, `ManualWeek`, `ManualMonth`, `ManualYear`). Toute violation est rejetée avec une erreur explicite.

**R3 — Unicité du nom (frontend + backend)** : Deux comptes actifs ne peuvent pas porter le même nom (comparaison sur le nom normalisé selon R1, casse ignorée). Toute tentative de création ou de modification aboutissant à un doublon est rejetée par le backend avec une erreur explicite. Les règles R1, R2 et R3 s'appliquent à la création comme à la modification.

**R4 — UpdateFrequency — liste fixe (frontend + backend)** : La fréquence de mise à jour est une liste fixe de cinq valeurs non personnalisables. Elle est purement informative : aucun automatisme n'est déclenché en production à ce stade. Un comportement actif est prévu dans une version future.

**R5 — Suppression des Holding en cascade (backend)** : La suppression d'un `Account` est définitive et irréversible. Elle entraîne la suppression de tous ses `Holding` associés. Les `Asset` eux-mêmes ne sont pas affectés.

**R6 — Suppression des transactions en cascade (backend)** : La suppression d'un `Account` entraîne la suppression de toutes les transactions associées à ce compte. Les `Asset` référencés par ces transactions ne sont pas affectés.

> ⚠️ **Dépendance** : R6 n'est implémentable qu'une fois la feature Transaction disponible (`docs/transaction.md`). Tant que cette table n'existe pas en base, seul le cascade sur les `Holding` (R5) est actif.

### Frontend

**R7 — Valeur par défaut UpdateFrequency (frontend)** : Le formulaire de création pré-sélectionne `ManualMonth` comme fréquence par défaut.

**R8 — Tableau des comptes (frontend)** : Le tableau affiche les colonnes suivantes, trié par défaut par Nom ascendant :

| Colonne   | Contenu                               | Triable                                                                                                        |
| --------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Nom       | `account.name`                        | Oui                                                                                                            |
| Fréquence | Libellé lisible de `update_frequency` | Oui — tri sur l'ordre logique de l'énumération : Automatic → ManualDay → ManualWeek → ManualMonth → ManualYear |
| Actions   | Bouton Éditer + Bouton Supprimer      | Non                                                                                                            |

Un en-tête affiche le titre « Comptes », le nombre total de comptes et un champ de recherche filtrant par nom en temps réel (correspondance partielle, insensible à la casse).

**R9 — Comportement du tri (frontend)** : Cliquer sur l'en-tête d'une colonne triable bascule entre l'ordre ascendant et descendant. Un indicateur visuel sur l'en-tête reflète le tri actif et son sens.

**R10 — Aucun résultat de recherche (frontend)** : Si la recherche ne correspond à aucun compte, le tableau affiche un message distinct de l'état vide (ex. « Aucun résultat pour cette recherche. »). Ce message n'invite pas à créer un compte.

**R11 — État vide du tableau (frontend)** : Si aucun `Account` n'existe, le tableau affiche un état vide explicite invitant l'utilisateur à créer son premier compte via le FAB.

**R12 — États de chargement et d'erreur (frontend)** : Le tableau des comptes expose un état de chargement et un état d'erreur avec bouton Réessayer si le chargement initial échoue.

**R13 — Erreurs backend (frontend)** : La modal de création et la modal de modification restent ouvertes en cas d'erreur backend et n'affichent le succès (fermeture + rafraîchissement) qu'après un retour positif. Tout échec (doublon, erreur réseau, erreur backend) affiche un message d'erreur inline dans la modal ou la dialog active.

**R14 — Création via FAB (frontend)** : Un FAB flottant en bas à droite ouvre une modal de création avec un champ Nom (requis) et un sélecteur `UpdateFrequency`. La soumission est bloquée si le nom est vide ou ne contient que des espaces. Après création, la modal se ferme et le tableau se rafraîchit. Si le backend retourne une erreur, un message d'erreur est affiché inline dans la modal (R13).

**R15 — Modification (frontend)** : Le bouton Éditer ouvre une modal avec les champs Nom et `UpdateFrequency` pré-remplis. Après sauvegarde, la modal se ferme et le tableau se rafraîchit. Si le backend retourne une erreur, un message d'erreur est affiché inline (R13).

**R16 — Suppression d'un compte vide (frontend)** : Si l'`Account` ne contient aucun `Holding`, le bouton Supprimer ouvre une dialog de confirmation standard. La confirmation déclenche la suppression (R5, R6) ; après suppression, la dialog se ferme et le tableau se rafraîchit.

**R17 — Suppression d'un compte non vide (frontend)** : Si l'`Account` contient au moins un `Holding`, le bouton Supprimer ouvre une dialog de confirmation renforcée indiquant le nombre de `Holding` et de transactions concernés, et précisant que toutes ces données seront définitivement supprimées avec le compte. La confirmation déclenche la suppression (R5, R6) ; après suppression, la dialog se ferme et le tableau se rafraîchit.

> ⚠️ **Dépendance** : R17 n'est implémentable qu'une fois la feature Holding disponible (`docs/transaction.md`). Tant que cette feature n'existe pas, R16 s'applique pour toutes les suppressions (dialog standard).

---

## Workflow

```
[Utilisateur ouvre « Comptes »]
  → Tableau (tri défaut : Nom asc) + FAB
          │
          ├─ [Recherche] → Filtre temps réel par nom (R8)
          │                → Aucun résultat → message distinct (R10)
          │
          ├─ [Clic en-tête] → Tri asc/desc + indicateur visuel (R9)
          │
          ├─ [FAB] → Modal création (Nom + Fréquence, défaut ManualMonth)
          │            → Erreur inline si nom vide ou doublon (R13)
          │            → Compte créé → modal fermée → tableau rafraîchi
          │
          ├─ [Éditer] → Modal édition (Nom + Fréquence pré-remplis)
          │            → Erreur inline si doublon (R13)
          │            → Modification → modal fermée → tableau rafraîchi
          │
          └─ [Supprimer]
              ├─ Si compte vide → Dialog standard (R16) → Confirmation → Suppression (R5, R6)
              └─ Si compte non vide → Dialog renforcée (R17, nb Holding + transactions)
                                      → Confirmation → Suppression + cascade (R5, R6)
```

---

## Maquette UX

### Point d'entrée

**Comptes** — item du drawer de navigation principal.

### Composant principal

Page avec tableau pleine largeur, trié par défaut par Nom ascendant. FAB flottant en bas à droite. Bouton Éditer et Supprimer sur chaque ligne.

### États

- **Vide** : message invitant à créer le premier compte via le FAB (R11)
- **Chargement** : indicateur de chargement dans le tableau (R12)
- **Erreur de chargement** : message d'erreur + bouton Réessayer (R12)
- **Aucun résultat** : message distinct de l'état vide (R10)
- **Erreur inline** : message affiché dans la modal ou la dialog active (R13)
- **Confirmation suppression compte vide** : dialog de confirmation standard (R16)
- **Confirmation suppression compte non vide** : dialog « Ce compte contient X position(s) et Y transaction(s). Toutes ces données seront définitivement supprimées. » (R17)

### Flux utilisateur

1. L'utilisateur ouvre la page Comptes.
2. Il clique sur le FAB → modal → saisit un nom et choisit une fréquence → soumet → compte créé.
3. Il clique sur Éditer → modal pré-remplie → modifie → sauvegarde → modal fermée.
4. Il clique sur Supprimer (compte vide) → dialog standard → confirme → compte supprimé.
5. Il clique sur Supprimer (compte avec positions) → dialog renforcée → confirme → compte + positions + transactions supprimés.

---

### Navigation

**ACC-010 — Account details navigation (frontend)**: Clicking on an account table row (excluding action buttons) navigates the user to the Account Details view.

---

## Future Improvements

### Migration to Standard Spec Format

- **Full migration required**: This specification currently uses the legacy "Rxx" format in French. A full migration to the new standard (TRIGRAM-NNN format, English language, atomic rules) is required to ensure consistency across the documentation suite.
- **Trigram**: Future updates should use the `ACC` trigram.

### Feature expansion

- **Archiving**: Implement account archiving (soft-delete) instead of permanent deletion to preserve transaction history.
- **Success Feedback**: Implement snackbar notifications for all mutations.

---

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

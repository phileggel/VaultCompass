# Règles métier — Gestion des catégories d'assets

## Contexte

Une catégorie (`AssetCategory`) est un regroupement libre défini par l'utilisateur pour organiser ses assets (ex. « Actions US », « Immo Europe », « Obligations court terme »). Ce n'est pas une taxonomie fixe : l'utilisateur crée, renomme et supprime ses propres catégories. Les catégories sont utilisées dans le dashboard de performance pour agréger les valeurs par groupe. Une catégorie système (`default-uncategorized`) existe en permanence comme valeur de repli lorsqu'aucune catégorie n'est choisie.

---

## Définition des champs d'une catégorie

| Champ  | Signification                                                                                                                           |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `id`   | Identifiant unique généré à la création (UUID). La catégorie système a l'id fixe `default-uncategorized`.                               |
| `name` | Nom lisible défini par l'utilisateur (ex. « Actions US », « Immo Europe »). Unique parmi toutes les catégories actives (casse ignorée). |

---

## Règles métier

### Backend

**R1 — Champs requis (backend)** : Une catégorie est valide si son `name` est non vide et unique parmi toutes les catégories existantes (casse ignorée). Le backend rejette une création ou modification avec un nom déjà utilisé.

**R2 — Catégorie système non supprimable et non renommable (backend)** : La catégorie `default-uncategorized` (id : `default-uncategorized`) est une catégorie système pré-seedée. Elle ne peut pas être supprimée ni renommée. Toute tentative est rejetée par le backend avec une erreur explicite.

**R3 — Suppression avec réassignation atomique (backend)** : La suppression d'une catégorie non système réassigne tous les assets liés vers `default-uncategorized` dans la même transaction SQL. Si la réassignation échoue, la suppression est annulée. La suppression est toujours autorisée (aucun blocage préalable).

### Frontend

**R4 — Tableau des catégories — colonnes (frontend)** : Le tableau affiche les colonnes suivantes, trié par défaut par Nom ascendant :

| Colonne | Contenu                                                                                | Triable |
| ------- | -------------------------------------------------------------------------------------- | ------- |
| Nom     | `category.name` + badge « Défaut » si catégorie système                                | Oui     |
| Actions | Bouton Éditer (toujours visible) + Bouton Supprimer (masqué pour la catégorie système) | Non     |

Un en-tête affiche le titre « Catégories », le nombre total de catégories et un champ de recherche filtrant par nom.

**R5 — Visibilité de la catégorie système (frontend)** : `default-uncategorized` est visible dans la liste avec un badge traduit (« Défaut » / « Default »). Le bouton Supprimer est visible mais désactivé (disabled). Le bouton Éditer est visible mais désactivé (disabled).

**R6 — Création via FAB (frontend)** : Un FAB flottant en bas à droite ouvre une modal de création avec un unique champ Nom (requis). La soumission est bloquée si le nom est vide. Si le backend retourne une erreur de doublon, un message d'erreur est affiché inline dans la modal.

**R7 — Modification (frontend)** : Le bouton Éditer ouvre une modal avec le champ Nom pré-rempli. Après sauvegarde, la modal se ferme et le tableau se rafraîchit. Si le backend retourne une erreur (doublon ou catégorie système), un message d'erreur est affiché inline.

**R8 — Suppression (frontend)** : Le bouton Supprimer ouvre une dialog de confirmation indiquant que les assets liés seront déplacés vers « Non catégorisé ». La confirmation déclenche la suppression et la réassignation atomique (R3).

**R9 — États d'erreur (frontend)** : Tout échec d'appel backend (création, modification, suppression) affiche un message d'erreur inline dans la modal ou dialog active. Le tableau expose un état d'erreur avec bouton Réessayer si le chargement initial échoue.

---

## Workflow

```
[Utilisateur ouvre « Catégories »]
  → Tableau (tri défaut : Nom asc) + FAB
          │
          ├─ [Recherche] → Filtre temps réel par nom
          ├─ [Clic en-tête Nom] → Tri ascendant/descendant
          │
          ├─ [FAB] → Modal création (champ Nom)
          │            → Erreur inline si nom vide ou doublon
          │            → Catégorie créée → modal fermée → tableau rafraîchi
          │
          ├─ [Éditer] → Modal édition (Nom pré-rempli)
          │   [disabled si système]
          │            → Erreur inline si doublon ou système
          │            → Modification → modal fermée → tableau rafraîchi
          │
          └─ [Supprimer] → Dialog (mention réassignation vers défaut)
          [disabled si système]
                            → Confirmation → Suppression + réassignation atomique
```

---

## Maquette UX

### Point d'entrée

**Catégories** — item du drawer de navigation principal.

### Composant principal

Page avec tableau pleine largeur, trié par défaut par Nom ascendant. FAB flottant en bas à droite. Bouton Éditer et Supprimer sur chaque ligne, tous deux disabled pour la catégorie système.

### États

- **Vide** : impossible (la catégorie système est toujours présente)
- **Chargement** : Indicateur de chargement dans le tableau
- **Erreur de chargement** : Message d'erreur + bouton Réessayer
- **Erreur doublon** : Message inline dans la modal « Ce nom est déjà utilisé. »
- **Confirmation suppression** : Dialog « Les assets associés seront déplacés vers "Non catégorisé". »
- **Erreur backend** : Message d'erreur inline dans la modal ou dialog active

### Flux utilisateur

1. L'utilisateur ouvre la page Catégories.
2. La catégorie système apparaît avec badge « Défaut », boutons Éditer et Supprimer tous deux disabled.
3. Il clique sur le FAB → modal → saisit un nom → soumet → catégorie créée.
4. Il clique sur Éditer (catégorie custom) → modal pré-remplie → modifie → sauvegarde.
5. Il clique sur Supprimer → dialog (mention réassignation) → il confirme → suppression + réassignation atomique.

---

## Waivers de test

**R5, R8, R9 — rendu conditionnel de `CategoryTable` non testé** : Ces règles impliquent uniquement du rendu conditionnel React (badge/disabled/hidden, ouverture de dialog, affichage d'erreur). La logique métier sous-jacente est couverte par les tests existants :
- R5 : `isSystemCategory()` est une fonction pure testée indirectement via les tests de service backend (R2) ;
- R8 : l'opération de suppression atomique est couverte par `delete_category_reassigns_assets_to_default` (service.rs) ;
- R9 : les chemins d'erreur Add/Edit sont couverts par `useAddCategory.test.ts` et `useEditCategoryModal.test.ts`.

Un test de composant RTL complet nécessiterait de mocker le store Zustand et le gateway — coût disproportionné pour vérifier des ternaires JSX sans logique.

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

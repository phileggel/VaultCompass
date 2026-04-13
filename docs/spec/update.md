# Règles métier — Mise à jour de l'application

## Contexte

L'application est distribuée sous forme d'exécutable desktop. Lorsqu'une nouvelle version est publiée, l'utilisateur doit en être informé afin de pouvoir l'installer sans avoir à vérifier manuellement. Cette feature couvre la détection d'une mise à jour disponible, la proposition à l'utilisateur, le téléchargement, l'installation, et les garanties sur les données utilisateur.

---

## Règles métier

### Découverte

**R1 — Vérification automatique au démarrage (backend)** : À chaque lancement, une vérification est effectuée en arrière-plan pour détecter si une nouvelle version est disponible. Elle démarre une fois l'interface entièrement chargée.

**R2 — Absence de notification si aucune mise à jour (frontend)** : Si la vérification R1 ne détecte aucune nouvelle version, rien n'est affiché à l'utilisateur et l'application reste dans son état normal.

### Notification

**R3 — Contenu de la bannière de mise à jour (frontend)** : Si une nouvelle version est détectée, une bannière fixe s'affiche dans le shell de l'application, indiquant le numéro de version disponible et proposant deux actions : « Installer » et « Ignorer ».

**R4 — Persistance de la bannière (frontend)** : La bannière est intégrée au layout permanent du shell et reste visible sur toutes les pages de l'application sans interrompre la navigation.

**R5 — Fermeture de la bannière par l'utilisateur (frontend)** : Cliquer sur « Ignorer » ou sur le bouton × ferme la bannière. Cela déclenche le comportement de report décrit en R19.

### Téléchargement

**R6 — Déclenchement du téléchargement (frontend + backend)** : Lorsque l'utilisateur clique sur « Installer », le téléchargement de la mise à jour démarre en arrière-plan.

**R7 — Non-blocage pendant le téléchargement (frontend)** : Pendant le téléchargement, l'application reste entièrement navigable.

**R8 — Progression dans la bannière (frontend)** : Pendant le téléchargement, la bannière indique la progression. Le bouton « Installer » est remplacé par l'indicateur de progression et ne peut pas être re-déclenché.

**R9 — Intégrité du téléchargement (backend)** : Avant d'autoriser l'installation, l'application vérifie l'intégrité du fichier téléchargé par une somme de contrôle. Si la vérification échoue, le fichier est considéré comme corrompu et le flux d'erreur R23 s'applique.

**R10 — Téléchargement concurrent (backend)** : Si une nouvelle version est publiée alors qu'un téléchargement est déjà en cours, le téléchargement en cours se poursuit sans interruption. La version plus récente sera proposée au prochain lancement conformément à R20.

### Prêt à installer

**R11 — Contenu de la bannière état "Prêt à installer" (frontend)** : Lorsque le téléchargement est terminé et l'intégrité vérifiée, la bannière affiche « Prêt à installer » et un bouton « Redémarrer maintenant ».

**R12 — Bannière non-dismissible après téléchargement (frontend)** : Dans l'état "Prêt à installer", la bannière ne propose plus de bouton × ni d'option « Ignorer ». Elle reste visible jusqu'au redémarrage ou à la fermeture de l'application.

### Installation

**R13 — Redémarrage et installation (frontend + backend)** : Lorsque l'utilisateur clique sur « Redémarrer maintenant », la mise à jour est installée et l'application redémarre automatiquement.

**R14 — Conservation des données utilisateur (backend)** : La mise à jour remplace uniquement l'exécutable de l'application. Toutes les données utilisateur (assets, comptes, catégories, prix, opérations) sont conservées intégralement après le redémarrage.

**R15 — Compatibilité descendante (backend)** : Chaque nouvelle version garantit la compatibilité avec les données produites par toute version antérieure. Aucune mise à jour ne peut introduire un changement incompatible avec un schéma ou des données existants. Les migrations R16 ne peuvent qu'étendre ou adapter le schéma, jamais supprimer ou modifier des données de façon destructive.

**R16 — Migration automatique du schéma (backend)** : Si la nouvelle version introduit des changements de schéma de base de données, les migrations sont appliquées automatiquement au premier démarrage après la mise à jour, avant que l'interface ne soit accessible. Si plusieurs versions ont été sautées, toutes les migrations intermédiaires sont appliquées dans l'ordre, sans exception.

**R17 — Écran de chargement pendant les migrations (frontend)** : Pendant la phase de migration R16, un écran de chargement est affiché avec un message indiquant que la mise à jour de la base de données est en cours.

**R18 — Échec de migration (backend)** : Si une migration échoue au démarrage, l'application affiche un message d'erreur critique et refuse de démarrer afin de protéger l'intégrité des données. L'utilisateur est invité à contacter le support.

### Report

**R19 — Report au prochain lancement (frontend)** : Si l'utilisateur ferme la bannière R3 (par « Ignorer » ou ×), ou ferme l'application depuis l'état R11 sans avoir redémarré, la mise à jour est proposée à nouveau au prochain lancement.

**R20 — Priorité à la version la plus récente (backend)** : Si une version encore plus récente est disponible au moment du prochain lancement, c'est cette version qui est proposée, et non la version précédemment ignorée.

### Erreurs

**R21 — Erreur de vérification silencieuse (frontend)** : Si la vérification au démarrage échoue (pas de réseau, serveur indisponible), aucune notification n'est affichée et l'application démarre normalement.

**R22 — Log des erreurs de vérification (backend)** : Toute erreur survenant lors de la vérification au démarrage est consignée dans les logs de l'application.

**R23 — Affichage de l'erreur de téléchargement (frontend)** : Si le téléchargement échoue (erreur réseau, espace disque insuffisant, ou toute autre cause), ou si le checksum R9 échoue, la bannière affiche un message d'erreur et un bouton « Réessayer ».

**R24 — Action Réessayer (frontend + backend)** : Cliquer sur « Réessayer » relance le téléchargement depuis le début.

### Vérification manuelle

**R25 — Point d'entrée de vérification manuelle (frontend)** : La page « À propos » expose le numéro de version actuelle et un bouton « Vérifier les mises à jour ». Lorsque l'utilisateur clique sur ce bouton, une vérification est déclenchée selon le même mécanisme que R1.

**R26 — État de chargement de la vérification manuelle (frontend)** : Pendant la vérification déclenchée par R25, le bouton « Vérifier les mises à jour » est désactivé et affiche un indicateur de chargement afin de prévenir les déclenchements multiples.

**R27 — Résultat de la vérification manuelle (frontend)** : À l'issue de la vérification R25 : si une mise à jour est disponible, la bannière R3 s'affiche ; si aucune mise à jour n'est disponible, un message sur la page « À propos » confirme que l'application est à jour.

---

## Workflow

```
[Démarrage de l'app → interface chargée]           [Page À propos → bouton "Vérifier"]
  → Vérification en arrière-plan (R1)                → Vérification + spinner (R25, R26)
        │                                                   │ (même suite que R1)
        ├─ Pas de mise à jour → rien affiché (R2)    ──────┤
        │                                             └─ À jour → message confirmatif (R27)
        ├─ Erreur réseau/serveur → log silencieux, rien affiché (R21, R22)
        │
        └─ Nouvelle version disponible
              → Bannière : "Version X.Y.Z disponible" + [Installer] [Ignorer] (R3, R4)
                    │
                    ├─ [Ignorer / ×] → bannière disparaît, reproposé au prochain lancement (R5, R19)
                    │
                    └─ [Installer] → téléchargement en arrière-plan, app navigable (R6, R7)
                          → Progression visible dans la bannière (R8)
                          → Si version plus récente publiée → on continue (R10)
                                │
                                ├─ Échec → bannière erreur + [Réessayer] (R23)
                                │          [Réessayer] → repart depuis le début (R24)
                                │
                                └─ Succès → vérification checksum (R9)
                                        │
                                        ├─ Checksum KO → bannière erreur + [Réessayer] (R23, R24)
                                        │
                                        └─ Checksum OK → "Prêt à installer" + [Redémarrer] (R11)
                                                         bannière persistante, non dismissible (R12)
                                                │
                                                ├─ [Fermeture de l'app] → reproposé au prochain lancement (R19)
                                                │
                                                └─ [Redémarrer maintenant] (R13)
                                                      → Données conservées (R14)
                                                      → Migration DB si nécessaire (R16, R17)
                                                           ├─ Échec → erreur critique, app bloquée (R18)
                                                           └─ Succès → installation + redémarrage (R13)
```

---

## Maquette UX

### Points d'entrée

Deux points d'entrée :

1. **Automatique** — déclenché au démarrage, une fois l'interface entièrement chargée (R1).
2. **Manuel** — bouton « Vérifier les mises à jour » sur la page « À propos » (R25).

### Composant principal

Bannière fixe intégrée au shell de l'application (en-tête ou pied de page), visible sur toutes les pages. Elle n'apparaît que lorsqu'une mise à jour est en cours ou disponible, et disparaît une fois l'application redémarrée. Ce n'est pas un dialog ni une notification éphémère : la bannière fait partie du layout permanent.

### États de la bannière

- **Absente** : aucune mise à jour détectée — la bannière n'est pas rendue (R2)
- **Mise à jour disponible** : "Version X.Y.Z disponible" + boutons « Installer » et « Ignorer » (R3)
- **Téléchargement en cours** : indicateur de progression, bouton « Installer » remplacé (R8)
- **Prêt à installer** : "Prêt à installer" + bouton « Redémarrer maintenant » — non dismissible (R11, R12)
- **Erreur de téléchargement** : message d'erreur + bouton « Réessayer » (R23)
- **Migration en cours** : écran de chargement hors bannière ("Mise à jour de la base de données…") (R17)
- **Erreur de migration** : message d'erreur critique, application bloquée au démarrage, hors bannière (R18)
- **À jour** : message contextuel sur la page « À propos », hors bannière (R27)

### Flux utilisateur

1. L'application se lance, l'interface s'affiche, puis une vérification s'effectue en arrière-plan.
2. Si une version plus récente est disponible, une bannière apparaît avec le numéro de version.
3. L'utilisateur clique sur « Installer » → téléchargement en arrière-plan, progression visible, navigation libre.
4. Téléchargement terminé → bannière persistante « Prêt à installer » + bouton « Redémarrer maintenant ».
5. L'utilisateur clique sur « Redémarrer maintenant » quand il est prêt → migrations éventuelles → installation → redémarrage.

---

## Features futures

- **Mises à jour critiques** : certaines versions pourraient être marquées comme critiques (faille de sécurité, corruption de données), imposant une mise à jour obligatoire avant que l'application soit utilisable. Non inclus dans cette version.
- **Notes de version** : au premier démarrage après une mise à jour, afficher les notes de version (changelog) de la nouvelle version installée. Non inclus dans cette version.

---

## Questions ouvertes

Aucune — toutes les questions ont été tranchées.

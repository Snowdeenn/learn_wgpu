# Documentation de Velum

Cette documentation décrit l'API publique présente dans ce dépôt. Elle ne décrit
pas une API future : les notes dans `note/` sont des documents de conception et
peuvent contenir des propositions non implémentées. Les signatures du code et
la documentation Rust générée restent la référence exhaustive.

Velum est une bibliothèque de rendu Rust, construite sur wgpu. Elle fournit un
backend, des scènes persistantes, un ordre explicite de passes, des commandes
de dessin et des effets raccordables. Elle ne décide ni du gameplay, ni de la
navigation de l'application, ni de notions imposées de monde/VFX/HUD.

## Parcours conseillé

1. [Architecture et flux](architecture.md) : vocabulaire et responsabilités.
2. [Démarrage et cycle de frame](demarrage.md) : initialisation et rendu.
3. [Scènes et SceneRenderer](scene-renderer.md) : stockage, planning, activation.
4. [Ressources et données](ressources.md) : propriété, partage, handles et buffers.
5. [Passes et opérations](passes.md) : rendu, compute et intégrations directes.
6. [Commandes, phases et dessin](dessin.md) : formes, sprites, tri et batching.
7. [Effets et connexions](effets.md) : paramètres, relais et validations.
8. [Créer un effet personnalisé](effet-personnalise.md) : exemple complet.
9. [Pipelines, shaders et bindings](pipelines.md) : configuration GPU.
10. [Texte, caméra et particules](texte-camera-particules.md).
11. [Erreurs, diagnostic et limites](diagnostic.md).
12. [Index de l'API publique](reference.md).

## Exemples complets

- [Scène avec une source, un flou et une présentation](examples/scene_minimale.rs).
- [Effet personnalisé possédant ses ressources et sa passe](examples/effet_custom.rs).
- [Pipeline et opération avec shader WGSL intégré et uniforme](examples/pipeline_custom.rs).

Ces fichiers sont des modules Rust compilés par `tests/documentation.rs`.
La compilation vérifie les signatures et les imports ; elle ne remplace pas
l'exécution GPU. Leurs fonctions de construction nécessitent un backend déjà
initialisé. Les autres extraits des guides sont des fragments contextualisés :
les variables comme `format`, `resolution` et `frame` viennent de l'hôte.

## Vérification et référence générée

Depuis la racine du dépôt :

```powershell
cargo check --offline --all-targets
cargo test --offline --lib --tests
cargo test --offline --doc
cargo test --offline --lib -- --ignored
cargo doc --offline --no-deps
```

Le test ignoré nécessite un adaptateur GPU. La documentation Rust est produite
dans `target/doc/velum/index.html`. Les commandes peuvent omettre `--offline`
si les dépendances ne sont pas encore disponibles localement.

Le paquet Cargo s'appelle actuellement `learn_wgpu`, tandis que la bibliothèque
importée dans le code s'appelle `velum`. Ce détail est important pour une
dépendance depuis un autre dépôt ; voir le guide de démarrage.

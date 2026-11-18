# Scènes, données et SceneRenderer

## Le rôle de Velum

```rust
use velum::{Scene, SceneRenderer, RenderData, Velum};

let mut velum = Velum::new();
let menu = velum.add_scene(Scene {
    renderer: SceneRenderer::new(),
    data: RenderData::new(),
});
velum.activate(menu)?;
```

`activate` vérifie l'identifiant et conserve l'ancienne sélection si la nouvelle
est invalide. `active_scene()` retourne l'ID sélectionné. `deactivate()` enlève
la sélection sans supprimer les données. `render` sans scène active retourne
`SceneError::NoActiveScene` : il ne produit pas automatiquement une image vide.

`scene(id)` et `scene_mut(id)` donnent un accès optionnel au conteneur. Une scène
inactive conserve ses ressources et son état. `remove_scene(id)` la retourne et
la désactive si nécessaire. Sa destruction réelle dépend du moment où la valeur
retournée est abandonnée.

## Passes locales

`add_pass(pass)` stocke la passe et l'ajoute immédiatement à la fin du planning.
Il retourne un `PassId`. Le builder suit la même règle :

```rust
let mut builder = velum::scene_renderer::renderer();
let pass_id = builder.add_pass(my_pass);
let renderer = builder.build();
```

Ce builder est mutable : `add_pass` retourne un ID, pas le builder. Ce n'est donc
pas un enchaînement `.add_pass(...).build()`. Après `build`, le renderer permet
encore `add_pass` et la gestion des effets.

## Effets : posséder puis programmer

```rust
let effect_id = renderer.add_effect(effect);
renderer.connect(effect_id, "source", source)?;
let effect_anchor = renderer.append_effect(effect_id)?;
```

`add_effect` transfère la propriété, mais ne programme aucune passe. L'effet est
actif par défaut. `append_effect` référence toutes ses passes dans leur ordre
interne. Il refuse une liste vide et une seconde insertion du même effet.

`schedule()` permet de consulter le planning, sans le modifier directement :

```rust
enum ScheduledPass {
    Local(PassId),
    Effect { effect: EffectId, pass: PassId },
}
```

Un ID de passe interne se résout dans la PassList de l'effet indiqué. Il ne faut
pas utiliser ce même ID comme passe locale. Les handles sont liés à leur stockage ;
ne pas les mélanger entre renderers. Leur provenance n'est pas uniformément
contrôlée par tous les stockages actuels.

## Déplacer des blocs

```rust
use velum::ScheduleAnchor;

renderer.insert_before(ScheduleAnchor::Local(final_pass), effect_anchor)?;
renderer.insert_after(ScheduleAnchor::Effect(first_effect), effect_anchor)?;
```

Le premier argument est la référence, le second est l'élément déplacé. Les deux
doivent déjà être programmés. Ces méthodes déplacent un bloc ; elles n'ajoutent
pas une nouvelle passe. Les étapes internes d'un effet restent contiguës.
Le planning précédent est restauré si le déplacement invalide les dépendances.

## Activation et paramètres

```rust
renderer.set_effect_active(effect_id, false)?;
renderer.effect_mut::<BlurEffect>(effect_id)?.settings.radius = 6;
```

La résolution du relais est expliquée dans [effets](effets.md). L'accès typé
vérifie l'existence et le type concret. Ne pas modifier la structure de la PassList
après `append_effect` : ses IDs et son ordre sont enregistrés dans le planning.

`activate_pass` et `deactivate_pass` concernent les passes locales. Actuellement,
un ID local absent est ignoré par ces deux méthodes ; elles ne renvoient pas
d'erreur. Désactiver une passe source ne résout pas ses consommateurs : une passe
locale n'a pas de contrat de relais comparable à celui d'un effet.

## Retrait et exécution

Reconnecter les consommateurs avant de retirer leur producteur :

```rust
renderer.connect(consumer, "source", replacement)?;
drop(renderer.remove_effect(producer)?);
```

`remove_effect` retire le bloc et retourne l'objet. Il ne réécrit pas les connexions
des autres effets ; les références restantes produisent une erreur à la validation.

`execute(frame, data)` prépare les effets puis parcourt le planning. Ne pas vider
ce planning à chaque image. Les paramètres et les données changent ; les passes
et leurs propriétaires persistent.

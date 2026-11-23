# Formes, sprites, commandes et phases

## Formats acceptés

Le backend peut dessiner les types de l'application qui implémentent ses traits :

- `Instance` : marqueur de données d'instance CPU.
- `ShapeInstance` : fournit un mesh enregistré, un RenderInstance et une clé de tri.
- `SpriteInstance` : fournit un Sprite, sa représentation RenderInstance et une clé.
- `TextInstance` : chemin dédié au texte, décrit dans le guide suivant.

`InstanceData<T>` est un Vec typé avec des accès Deref. `InstanceStorage` est un
stockage local de groupes identifiés, distinct des arènes globales de ressources.
Avec RenderData, on peut également stocker directement plusieurs InstanceData.

## RenderInstance

Le format GPU standard contient couleur RGBA, position 2D, échelle, rotation en
radians, profondeur, couche de texture et UV. Le champ `layer` est un indice de
texture array : ce n'est pas un ordre de scène. Le padding fait partie du layout.
Ne pas changer cette structure sans adapter les shaders correspondants.

Les traits permettent de produire ce format depuis des objets applicatifs sans
exposer leur représentation métier au backend. `sort_key()` utilise par défaut
un ordre explicite 0 et la profondeur de RenderInstance ; le surcharger évite
une conversion complète supplémentaire quand on connaît déjà la clé.

## Géométrie

`geometry::mesh::Mesh` permet d'ajouter vertices et indices. Les helpers
`create_rect_vertices`, `create_triangle_vertices`, `create_circle_vertices`
produisent des meshes CPU. `shape::register(None, mesh)` les enregistre au GPU et
retourne un propriétaire. Le mesh expose ensuite un ID aux instances.

`shape::Shape` est un type de forme prêt à employer. Ses conversions de profondeur
actuelles sont orientées 2D ; pour des règles spécifiques, implémenter le trait
sur son propre type. Le pipeline doit accepter le layout des vertices du mesh.

## Sprites et textures

`Sprite::Separate` utilise une texture indépendante ; `Array` désigne une couche ;
`Atlas` référence une région ; `Animated` conserve une plage de couches et son état
d'animation. `sprite::run_animated` actualise cet état sur un InstanceData mutable.
La cadence est actuellement déduite de la plage, pas d'une piste d'animation
générale avec timings individuels.

`load_texture` charge une image, `register_texture` retourne un propriétaire et
un layout de binding. Les variantes array et atlas ont leurs propres fonctions.
Ce chargement n'est pas un cache par chemin : charger deux fois un fichier ne
garantit pas une ressource partagée. L'application peut conserver un cache de
SharedResource si elle souhaite cette déduplication.

L'atlas conserve ses régions ; la résolution UV est effectuée par le cache
d'instances lors de la préparation. Une texture ou une région supprimée produit
une erreur dans les chemins de dessin vérifiés.

## Dessin direct

Après avoir lié un pipeline compatible dans une opération :

```rust
velum::backend::shape::draw_all(&mut objects, pass)?;
velum::backend::sprite::draw_all(&mut sprites, pass)?;
```

Ces fonctions dessinent dans l'ordre fourni. Les batchers regroupent uniquement
les voisins compatibles. `batch` crée un Vec de batches ; `batch_into` remplit
un Vec réutilisable. `draw` exécute un batch donné. Les gros groupes sont découpés
pour respecter les allocations du cache.

Le chemin sprite attend la convention actuelle : groupe 0 instances, groupe 1
typiquement caméra, groupe 2 texture/sampler, plus la géométrie et les indices
appropriés dans le contexte de pipeline. Le backend ne déduit pas les shaders.

## Rendu différé par commandes

Une DrawCommand possède une description CPU : pipeline effacé en type, géométrie,
RenderInstance, DrawSortKey et éventuelles substitutions de bindings.

```rust
let command = velum::backend::DrawCommand::shape(
    pipeline_id,
    mesh_id,
    instance,
    sort_key,
);
phase.push(command);
phase.execute(pass)?;
```

`DrawCommand::sprite` accepte un Sprite à la place du mesh. Les constructeurs
convertissent les PipelineId typés vers DrawPipeline. `with_bind_group` remplace
un groupe pour la commande. Les groupes réservés aux instances (0) et, pour les
sprites, à la texture (2), ne peuvent pas être remplacés ainsi.

## RenderPhase et tri

Une phase est un ensemble de commandes soumis à une seule politique :

```rust
let mut phase = velum::backend::RenderPhase::new(
    velum::backend::SortPolicy::BackToFront,
);
```

- PreserveOrder : ordre de soumission.
- DrawOrder : DrawSortKey.draw_order croissant.
- FrontToBack : profondeur croissante, proche d'abord.
- BackToFront : profondeur décroissante, loin d'abord.

Les égalités conservent l'ordre de soumission. Pour le tri de profondeur, la clé
doit être finie. En 3D une profondeur de tri doit dépendre de la vue courante ;
Velum ne calcule pas encore automatiquement cette clé depuis une caméra 3D.

La phase trie les commandes, pas les objets sources. Elle sépare d'abord les
états de pipeline/bindings/catégorie, puis utilise les batchers existants.
Une séquence texture A, B, A reste trois groupes. Des états différents ne sont
pas fusionnés pour diminuer artificiellement les draw calls.

`prepare()` trie sans encoder. `commands()` consulte les soumissions. `clear()`
abandonne les commandes. `execute()` trie, vérifie, dessine, puis vide les commandes
même en cas d'erreur, en conservant les capacités de travail. Conserver la phase
dans l'opération évite de recréer ces vecteurs à chaque image.

Le tri ne configure ni blending ni test/écriture de profondeur. Des transparents
triés avec un pipeline opaque ne deviennent pas transparents. Le tri par objet
ne résout pas non plus les intersections générales de transparence 3D.

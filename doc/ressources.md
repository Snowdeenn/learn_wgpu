# Ressources GPU et données CPU

## Accès depuis une application

`velum::registries` expose les registres globaux : `context`, `buffer`,
`texture`, `shape`, `pipeline`, `compute`, `target`, `binding_set`, `text`
et `particle`. Ce sont les mêmes modules et les mêmes stockages que ceux
utilisés par le backend, partagés entre les scènes et les instances de `Velum`.

```rust
use velum::registries::{context, target, texture};

// Après context::init(&adapter) :
let owner = target::register(None, descriptor);
let snapshot = target::get(owner.id()).unwrap();
// Conserver owner dans l'application ou dans les données d'une scène.
```

Les ressources GPU nécessitent l'initialisation du contexte par défaut avec
`context::init(&adapter)`. L'application peut ensuite enregistrer ses ressources
et transmettre leurs IDs aux commandes de rendu de Velum.

## Handle, propriétaire, instantané

Un handle copiable (`RenderTargetId`, `PipelineId<T>`, etc.) est un accès logique.
`OwnedResource<H>` en est le propriétaire exclusif, non clonable. Son `Drop`
appelle `ResourceHandle::release` pour retirer la ressource du stockage global.

```rust
let owner = target::register(None, descriptor);
let id = owner.id();
data.insert(owner); // La scène conserve la propriété.
```

Ne pas écrire `let id = target::register(None, descriptor).id();` : le propriétaire
temporaire est détruit à la fin de l'instruction et l'ID devient invalide.
Ne pas recréer plusieurs `OwnedResource::new(id)` sur des copies du même ID :
le constructeur ne garantit pas l'unicité à ta place.

Pour partager la propriété :

```rust
let shared = owner.share();
world_data.insert(shared.clone());
menu_data.insert(shared);
```

`share()` consomme le propriétaire. Le dernier `SharedResource` détruit retire
la ressource. Copier seulement `.id()` ne prolonge jamais sa durée de vie.

Un `Arc<RenderTarget>` obtenu par `target::get` est un instantané de la ressource
GPU résolue, pas un propriétaire de son inscription dans le registre. Un resize
peut remplacer cette inscription tout en laissant l'ancien Arc vivant.

## Ce qui est enregistré

Les fonctions de création/enregistrement retournent des propriétaires pour les
buffers, textures/arrays/atlas, meshes, pipelines graphiques/compute, cibles,
ensembles de bindings, textes et effets de particules. Les régions d'un atlas
appartiennent à l'atlas ; elles ne constituent pas des propriétaires indépendants.

Les fonctions de recherche retournent un accès à une ressource existante, pas un
nouvel OwnedResource. Les objets wgpu bruts (`Sampler`, shader, texture brute)
ont leur propre durée de vie Rust ; ils ne passent pas tous par une arène Velum.

## RenderData

```rust
use velum::RenderData;

let mut data = RenderData::new();
let position = data.insert([10.0_f32, 20.0]);
data.require_mut(position)?[0] += 1.0;
let value = data.require(position)?;
```

Le stockage utilise une arène de `Box<dyn Any>`. Plusieurs valeurs du même type
peuvent coexister. `RenderDataId<T>` est copiable sans imposer `T: Copy`.
`get/get_mut` retournent Option ; `require/require_mut` retournent une erreur
contenant le type attendu et le handle. `remove` retourne la valeur et invalide
son ID dans ce stockage.

Le type est effacé à l'intérieur, mais les accès publics sont typés. Le conteneur
ne sait pas dessiner un type arbitraire : les opérations choisissent ce qu'elles
lisent et les traits du backend définissent les formats de dessin acceptés.

RenderData n'est pas le conteneur `core::Resources`. Ce dernier stocke une valeur
par type dans des RefCell et sert à l'injection de dépendances côté application.
Ses accès peuvent paniquer en cas d'absence ou de conflit d'emprunt. RenderData
est passé explicitement aux opérations.

## Buffers

```rust
use velum::backend::buffer;

let uniform = buffer::uniform::<[f32; 4]>(None, 1)?;
buffer::write_buffer(&uniform, 0, &[[1.0, 0.5, 0.0, 1.0]]);
```

`uniform`, `storage`, `storage_rw`, `storage_vertex`, `vertex`, `index` créent des
allocations typées. Les données doivent être `bytemuck::Pod`. La capacité est
exprimée en éléments ; l'offset de `write_buffer` est en octets.
L'allocateur actuel utilise des pages de 1 Mio ; une allocation logique plus
grande est refusée. Les dessins d'instances peuvent être découpés en chunks par
le backend ; cela ne rend pas toute allocation utilisateur arbitrairement grande.

`raw_region` retourne le buffer wgpu et sa région. `resolve_binding` fournit un
accès de binding vérifié. Les types `Uniform`, `StorageRead`, `StorageReadWrite`,
`Vertex`, `Index`, `StorageVertex` expriment l'usage ; les layouts WGSL doivent
toujours correspondre aux structures Rust, y compris alignement et padding.

Libérer une allocation ne rend pas nécessairement toute sa page au système :
les pages et le cache d'instances peuvent conserver de la mémoire pour réemploi.

## Cibles et resize

`TargetSize::Fixed` conserve sa taille. `TargetSize::Window` suit la fenêtre.
`Renderer::resize` remplace les cibles concernées et incrémente leur révision.
Les IDs restent les points d'accès ; résoudre à nouveau un ID après resize.

Une cible lue par un shader doit avoir `TEXTURE_BINDING`, une pièce jointe
`RENDER_ATTACHMENT`, une lecture CPU via copie `COPY_SRC`. Les formats et nombres
d'échantillons doivent correspondre au pipeline. `PassTarget::Surface` est une
destination de frame, pas un RenderTargetId utilisable comme entrée d'effet.

## Contextes

`context::init(adapter)` initialise le contexte par défaut une fois.
`context::create(adapter)` crée un contexte identifié possédé. Les accesseurs
`device`, `queue`, `context` et `texture_storage` acceptent `Option<ContextId>` :
None désigne le défaut.

Cette présence de contextes identifiés ne garantit pas que tous les helpers de
rendu soient indépendants du contexte par défaut. Plusieurs stockages et effets
utilisent encore des fonctions globales/default. Ne pas mélanger des ressources
de devices différents ; l'intégration multidevice complète n'est pas établie.

Les fonctions de création des buffers, cibles, textures, samplers, maillages et groupes de textures prennent un `Option<ContextId>` en premier argument. Passez `None` pour le contexte par défaut ou `Some(contexte.id())` pour un contexte enregistré. Gardez le propriétaire du contexte vivant pendant l’utilisation de ses ressources.

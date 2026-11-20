# Géométries et matériaux décrits

Les modules `backend::geometry`, `backend::material` et `backend::object`
intègrent les descriptions de Prism au cycle d'image de Velum. Les scènes,
`RenderData`, les passes, le planning, les
effets, le texte et les propriétaires restent ceux de Velum. Les fonctions et
macros historiques restent disponibles avec leurs conventions de slots.

L'intégration est indépendante du dépôt Prism. Alpha n'est pas modifié.

## Construire et exécuter

`cargo run --example offscreen -- /tmp/velum-offscreen.png` produit une image et
exécute trois frames : flou actif, relais sans flou, puis resize avec flou.
L'exemple associe particules, phases, caméra monde, HUD logique avec DPI,
texte et composition dans un même planning Velum. Il n'ouvre pas de fenêtre.

`cargo run --example custom_instances` montre des objets CPU applicatifs
avec un payload GPU de 32 octets et un binding au groupe 3, binding 7.

Les nouvelles fonctions de création retournent `Result<OwnedResource<Id>>`.
Conserver le propriétaire ou appeler `share()` pour partager entre scènes.
Les IDs seuls ne retiennent aucune ressource. Les registres sont générationnels.

```rust
use velum::backend::{
    geometry::{self, Geometry},
    material::{self, Material},
    object::RenderObject,
};

// Après initialisation du contexte GPU et création du template :
let geometry = geometry::try_register(Geometry::generated(None, 0..3))?;
let material = material::try_register(Material {
    template: template.share(),
    bindings: vec![],
})?;
let object = RenderObject::new(geometry.id(), material.id());
```

Les géométries comportent des vertex buffers à slots explicites, des régions,
un index buffer optionnel (`Uint16` ou `Uint32`), une plage et un `base_vertex`.
`Geometry::generated` représente les sommets produits par le shader.
`backend::geometry::try_mesh` utilise le format `velum::geometry::mesh::Vertex`.
L'enregistrement historique d'un mesh prépare aussi cette géométrie ;
`backend::shape::geometry(id)` en fournit un propriétaire partagé.

`BufferRegion::from_shared` conserve l'allocation typée. `subregion` reçoit une
plage **relative à l'allocation**, vérifiée avant toute liaison. Le dessin
résout le slice borné, jamais le reste de la page. `BufferRegion::import`
accepte un buffer GPU dédié ; l'appelant garantit son device et ne doit pas
le détruire ni recycler sa mémoire pendant son utilisation.

`buffer::allocate_in` et `buffer::try_write_buffer` complètent les helpers
historiques avec contexte identifié et erreurs d'upload explicites. Les offsets
d'écriture sont en octets ; taille et offset doivent être multiples de quatre.
Les allocations typées conservent la taille maximale d'une page (1 Mio).

## Schémas, ressources et macros

`BindingEntry` est un `wgpu::BindGroupLayoutEntry`. Uniforms, storage, textures,
samplers, textures storage et arrays utilisent donc toutes les contraintes
wgpu, dont visibilité, dimension, accès, taille minimale et offset dynamique.
Les limites, features, layouts et compatibilité shader sont vérifiés par les
builders communs et les scopes d'erreur GPU. Les groupes absents restent absents.

```rust
use velum::backend::{bindings, BindingResource};
let schema = velum::binding_schema! {
    3 => [bindings::buffer_entry(
        7, wgpu::ShaderStages::VERTEX,
        wgpu::BufferBindingType::Storage { read_only: true }, 16, false,
    )],
};
let parameters = velum::try_binding_set! {
    context: None,
    group: 0,
    entries: [5 => BindingResource::uniform(None, &[1.0_f32; 4])?],
}?;
```

`try_binding_set!`, `try_material_template!`, `try_material!`,
`try_pipeline_context!(template: id)` et `try_compute_context!` délèguent aux
fonctions publiques correspondantes. Les variantes sans `try_` utilisent
`expect`. Le template accepte une description complète ou tous ses champs
nommés : aucun état de shader, vertex, blending, profondeur ou multisampling
n'est caché par la macro.

`pipeline_context!(template: id)` fournit un contexte compatible avec
`PipelineContext`, le stockage typé historique et `DrawPipeline`. Les macros
historiques de définition de contexte et d'opération restent utilisables.
`pipeline::try_compute` et `pipeline::dispatch` utilisent les mêmes schémas et
la même résolution ; le contexte satisfait aussi `ComputePipelineContext`.

La liaison graphique est centralisée dans `pipeline::bind_resolved` : pipeline,
groupes et offsets dynamiques, slots de vertex buffers et slices bornés, puis
index buffer et format. `backend::bind` adapte les contextes historiques à cette
fonction, et les objets préparés l'utilisent directement après validation.
La signature de `bind` et ses conventions historiques restent inchangées.

La résolution joint les contributions communes à la phase/contexte, celles
du matériau, celles de l'objet, puis le stream d'instances. Deux contributions
au même `(groupe, binding)` produisent une erreur. Un objet peut demander une
substitution explicite avec `BindingUse::overriding(id)`. Aucune priorité
silencieuse ne masque un conflit.

Les offsets dynamiques de `BindingUse::with_offsets` suivent l'ordre croissant
des bindings dynamiques du set. `BindingResource::Buffer { region, size }`
sépare la taille liée de la capacité : `offset_dynamique + size` doit rester
dans la région. Même aligné, un offset qui déborde la sous-allocation est rejeté.

`BindingResource::Target` conserve un propriétaire de cible et résout sa vue
actuelle à chaque préparation. Le resize et la révision sont ainsi respectés.
Les bindings `LegacyTexture` et `LegacySampler` conservent les ressources
historiques, y compris les arrays et atlas. Ils appartiennent au contexte par
défaut, comme les cibles et effets historiques.

## Instances applicatives, formes, sprites et particules

Un payload applicatif peut implémenter le marqueur historique `Instance` et le
nouveau `GpuInstance`, qui exige `Pod + Send + Sync` et décrit sa liaison :

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Color([f32; 4]);
impl velum::backend::instances::Instance for Color {}
impl velum::backend::GpuInstance for Color {
    fn binding() -> velum::backend::InstanceBinding {
        velum::backend::InstanceBinding::Storage { group: 3, binding: 7 }
    }
}
```

`InstanceStream::upload` transfère ce format sans conversion en `RenderInstance`.
La variante `Vertex` décrit le slot, le stride, le step mode et les locations.
Le template doit porter exactement le même layout. Les plages d'instances sont
vérifiées contre la capacité du stream. Comme en wgpu, les offsets des attributs
et les layouts storage doivent correspondre à la représentation WGSL ; `Pod`
garantit la validité des copies mémoire, pas la sémantique des champs du shader.

Pour séparer métadonnées CPU et payload GPU, implémenter `InstanceSource`
(`type Gpu`, `gpu_data`, `binding`) puis `ObjectInstance` (`geometry`,
`material`, `sort_key`). `instances::submit` extrait les données et crée les
objets, en découpant les uploads selon la capacité des pages.

`standard::try_template` fournit le format prêt à l'emploi de Velum. `Slots`
rend explicites les emplacements des instances, caméra, texture, sampler et
vertex buffer ; le shader et le schéma sont construits ensemble. Les groupes
peuvent être communs à plusieurs catégories si leurs bindings sont distincts.

`standard::submit_shapes` et `submit_sprites` acceptent les traits historiques
et soumettent des `RenderObject`. Les sprites résolvent UV d'atlas, couche et
frame animée ; la géométrie standard correspond au quad `0..6`. Les autres
géométries passent par des objets personnalisés. Les matériaux de textures sont
construits avec `standard::try_sprite_material` et un propriétaire partagé.

Les sorties de particules réutilisent ces adaptateurs. `output.shapes()` et
`output.sprites()` permettent plusieurs visites immuables après une seule
préparation. `RenderModuleExt::with_resource(mesh_ou_texture_partagé)` conserve
les propriétaires dans les modules, sans modifier leurs champs historiques.
Les traînées restent des sorties de formes et les sprites animés restent des
sorties de sprites. `velum::geometry::Tesselator` ajoute les formes CPU de Prism,
dont le nine-patch, directement au format de sommets Velum.

## Phases, passes et erreurs

`object::RenderPhase` garde les politiques Velum : soumission, ordre explicite,
profondeur croissante ou décroissante. Les tris sont stables. Après tri, seules
les plages d'instances voisines, contiguës et de mêmes géométrie, matériau,
stream et bindings sont fusionnées. Les commandes de la phase sont consommées
même en cas d'erreur ; les capacités des vecteurs sont réutilisées.

`object::prepare` est commun au dessin direct et au dessin par phases.
`object::PhaseOperation` récupère une phase par `RenderDataId` et s'insère dans
`RenderSequence` ou toute passe du planning. Les effets historiques opèrent
sur les cibles rendues par les objets, avec leurs connexions, activation et relais.

La politique choisie est la **propagation** : diagnostic contextualisé puis
`RenderError`, converti en `PassError`. Aucun dessin invalide n'est ignoré et
aucun matériau de secours n'est substitué. Après une erreur d'exécution,
abandonner l'encodeur entier. Une phase valide ses objets avant de les encoder.
Les remplacements valident une nouvelle description avant de remplacer l'entrée.

Les passes Velum vérifient formats couleur/profondeur et multisampling des
templates. Une intégration avec une passe wgpu externe peut déclarer ses
attachments avec `pipeline::AttachmentScope` pour obtenir le même contrôle.
Le blending est validé lors de la construction du pipeline. Le comportement
historique de profondeur par passe (clear à 1) est conservé.

## Cycle d'image, caméras et rapport

`Renderer::begin_frame` et `renderer::OffscreenFrame::new` commencent le rapport.
Créer la frame avant de préparer les ressources à mesurer. Ils conservent les
allocations encodées jusqu'à la soumission ou l'abandon, afin qu'une passe
ultérieure ne réutilise pas leur mémoire. `finish` soumet ; le drop abandonne
l'encodeur puis libère les allocations temporaires, le cache d'instances et le texte.
Une seule image peut être en préparation sur le thread de rendu.

Avec un encodeur brut, commencer par `report::begin`, puis après soumission
appeler `backend::submitted`, `text::submitted` et `report::submitted`. Sur abandon,
détruire d'abord tous les encodeurs et command buffers, puis appeler
`backend::discard`, `text::discard`, `report::abandoned`. Ne jamais soumettre ensuite
les anciennes commandes. Le cache historique d'instances reste géré par le
cycle `Frame` pour les consommateurs de l'ancienne API.

`standard::Cameras` propose deux bindings à un emplacement choisi. La caméra
monde utilise `Camera2D`; la caméra écran emploie les pixels logiques, origine
en haut à gauche, avec la même conversion DPI que `TextSpace::Screen`.
Le client conserve le suivi, la secousse, le zoom et les entrées.

`report::snapshot` rassemble :

| Mesure | Périmètre |
| --- | --- |
| Passes | Passes réellement ouvertes par les builders/listes Velum, activation respectée. |
| Draws, batches, instances | Formes, sprites, objets de rendu et fullscreen intégrés. |
| Dispatchs | `pipeline::dispatch`. |
| Uploads | Écritures typées de buffers, chargements de textures Velum et déclarations externes. |
| Allocations | Nouvelles allocations d'instances pendant l'image, capacité conservée du cache historique. |
| Buffers | Allocations actives, octets logiques et capacité des pages du contexte interrogé. |
| CPU | Préparation de chaque draw (hors tri/batching), encodage, appel complet d'exécution de passe (validation et activation comprises) ; ces périmètres peuvent se recouvrir. |
| Texte | Invocations Glyphon réussies ; ses draws et uploads internes restent opaques. |
| Diagnostics | Préparations, erreurs de dessins, de passes et d'effets instrumentées. |

`snapshot_in(Some(context))` choisit le contexte des statistiques de buffers.
Les compteurs de commandes appartiennent au thread de rendu. Les mesures de
capacité ne désignent jamais la mémoire physique GPU. Les commandes encodées
d'une image abandonnée restent comptées ; `FrameOutcome` distingue cet abandon.
Les opérations personnalisées contribuent avec `report::record_external`.

`None` désigne le contexte par défaut ; les créations et allocations peuvent
utiliser un `ContextId`. `context::register` / `init_from` acceptent un GPU déjà
créé par l'application. Les ressources historiques globales (texte, effets,
cibles, textures) conservent leur contexte par défaut. Les imports GPU bruts
doivent provenir du device déclaré.

Les templates sont immuables. Les bindings vers les cibles suivent leur révision.
Une suppression explicite invalide l'identifiant pour les prochains dessins,
même s'il reste un propriétaire partagé. Les snapshots déjà préparés gardent
leurs dépendances jusqu'à l'encodage et les allocations encodées restent
retenues jusqu'à la clôture de l'image. Garder aussi le propriétaire du
`ContextId` pendant la création et la résolution de ses ressources.
Aucun cache supplémentaire de layout, pipeline ou bind group, aucune vue ou
file d'uploads différés `FrameV2`, aucun rechargement automatique de shaders,
ni chaîne bloom/HDR/tone mapping ne sont ajoutés.

## Vérification

```sh
cargo test --offline --all-targets
cargo test --offline --doc
cargo test --offline --lib -- --ignored
cargo test --offline --test material_gpu -- --ignored
cargo run --offline --example offscreen -- /tmp/velum-offscreen.png
cargo run --offline --example custom_instances
```

La suite lit les pixels pour vérifier les instances applicatives, les slots
déplacés, les vertex buffers et indices sous-alloués, les indices 16 bits,
l'ordre alpha stable, le batching, les sprites/arrays/atlas, le resize et les
propriétaires de scènes. Elle vérifie aussi les offsets dynamiques, les mises
à jour invalides, les allocations gardées entre passes et l'abandon de frame.

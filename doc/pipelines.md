# Pipelines, shaders, samplers et bindings

## Pourquoi le pipeline reste explicite

Un pipeline décrit les shaders et états GPU : format de sortie, layouts de
bindings, formats de vertices, blending, profondeur, multisampling, etc.
Une RenderPhase choisit l'ordre des dessins, mais ne change pas ces états.

`PipelineContext` associe le pipeline wgpu à ses groupes, buffers de vertices et
buffer d'indices. `pipeline::bind` applique ces informations dans une passe.
Les méthodes du contexte retournent des références ; les ressources wgpu
résolues peuvent être conservées dans ce contexte.

`RenderPipelineBuilder::from_layout` construit un pipeline à partir d'un layout
et d'un shader. Les méthodes `with_*` configurent notamment les points d'entrée,
la couleur, le blending, la profondeur et le multisampling. `add_vertex_buffer*`
ajoute les layouts. `build` crée le pipeline wgpu ; les erreurs de validation
shader/layout peuvent encore être rapportées par wgpu, pas par un Result Velum.

## Définir un contexte

Pour un exemple complet, voir [pipeline_custom.rs](examples/pipeline_custom.rs) :
il construit un shader WGSL intégré au binaire, un uniforme de couleur, son binding,
le pipeline enregistré et une opération plein écran. Aucun buffer de vertices
n'est nécessaire : le vertex shader génère un triangle via vertex_index.
Le propriétaire de la couleur et celui du pipeline sont conservés dans RenderData.
L'exemple est un dessin direct ; son groupe 0 est un uniforme de couleur, pas le
layout d'instances réservé au chemin géométrique de RenderPhase.

```rust
velum::define_pipeline_context!(Sprite);
```

Cela génère un `SpritePipelineContext` qui implémente PipelineContext. La macro
`pipeline_context!` peut ensuite construire ce contexte avec les ressources et
layouts :

```rust
let context = velum::pipeline_context! {
    Sprite {
        shader: &shader,
        format: format,
        fragment: "fs_main",
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        bind_group: [
            (instances_layout,),
            (camera_layout, camera_group.clone()),
            (texture_layout,),
        ],
        vertex_buffer: [(Vertex::layout(), &vertices)],
        index_buffer: Some(&indices),
    }
};
let pipeline = velum::backend::pipeline::register(context);
```

Ce fragment suppose les variables GPU déjà créées et `Vertex` importé depuis
`geometry::mesh`. Les groupes sans valeur sont fournis au moment du dessin.
L'entrée vertex attendue par cette macro est `vs_main`. Conserver le propriétaire
`pipeline` ; transmettre son `.id()` aux commandes et opérations.

## Binding de buffers vérifiable

```rust
use velum::backend::buffer;

let camera = buffer::uniform::<[f32; 4]>(1)?;
let (layout, group) = velum::try_bind_group! {
    wgpu::ShaderStages::VERTEX_FRAGMENT => camera
}?;
```

Cet exemple montre seulement un uniforme vec4 ; une caméra complète utilise une
matrice au layout correspondant au shader. Les ressources sont numérotées dans
l'ordre de déclaration. `try_bind_group!` et `try_bind_group_layout!` propagent
les erreurs de résolution. Les variantes sans `try_` utilisent `expect` et peuvent
paniquer si la ressource est invalide.

Les builders `BindGroupLayoutBuilder` et `BindGroupBuilder` offrent les mêmes
outils à un niveau plus explicite : textures, samplers, buffers et régions.
`Bindable` décrit le type et la ressource à fournir à wgpu.

## Bindings de cibles redimensionnables

`binding_set::register(layout, entries)` crée un ensemble propriétaire. Les
`TextureBinding::Texture` référencent des RenderTargetId ; les samplers sont des
handles wgpu conservés dans les entrées.

```rust
let owner = binding_set::register(&layout, vec![
    TextureBinding::Texture { binding: 0, target: source_id },
    TextureBinding::Sampler { binding: 1, sampler: sampler.as_raw().clone() },
])?;
let group = binding_set::group(owner.id())?;
```

Résoudre le groupe juste avant son usage permet au backend de détecter un resize.
Conserver éternellement la valeur `group` empêche de profiter de ce rafraîchissement.
`update` change les entrées sous un même ID. Les contraintes de filtrage, format,
dimension, usage et binding shader restent à respecter.

## DrawPipeline

`PipelineId<T>::erase()` produit un DrawPipeline non propriétaire, capable de
retrouver un contexte graphique à travers un trait objet. C'est ce qui permet
à une RenderPhase de contenir des contextes concrets différents. Un pipeline
compute n'est pas automatiquement un DrawPipeline graphique.

Les overrides d'une DrawCommand font partie de sa compatibilité de batching.
Les slots non réservés sans valeur dans le contexte doivent être renseignés.
Un changement de binding peut donc séparer deux groupes, même avec le même mesh.

## Shaders et samplers

`backend::load_shader(label, path)` charge le WGSL depuis le disque, avec un
chemin relatif au répertoire de travail ou absolu. Les shaders fournis par
Velum sont dans `velum/assets/shaders/` et intégrés à la compilation avec
`include_str!` : les matériaux standards et effets n'ont pas besoin de fichiers
WGSL dans le répertoire de travail de l'application.

`SamplerBuilder` configure les filtres et modes d'adressage. Les helpers
`sampler::filtering` et `sampler::pixel_art` donnent des réglages courants.
Un sampler filtrant doit être associé à une texture et un layout filtrables.

## Compute

`ComputePipelineBuilder`, `ComputePipelineContext`, `ComputePipelineId` et
`define_compute_pipeline_context!` constituent le chemin compute. Enregistrer le
contexte retourne également un propriétaire. Une ComputeOperation lie ce contexte
et lance le dispatch dans la passe fournie. Les dépendances de buffers arbitraires
ne sont pas analysées automatiquement par le graphe des effets de textures.

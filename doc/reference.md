# Index de l'API publique

Cet index sert de carte des fonctionnalités. Pour les signatures complètes et les
champs de tous les builders, exécuter `cargo doc --offline --no-deps` et ouvrir
`target/doc/velum/index.html`. Un symbole accessible n'est pas nécessairement un
engagement de stabilité définitif ; les restrictions importantes sont signalées
dans les guides.

## Façade et scènes

- `Velum` : new/default, add_scene, scene/scene_mut, activate, active_scene,
  deactivate, remove_scene, render.
- `Scene`, `SceneId`, `SceneError` : conteneur, identité et erreurs de sélection.
- `SceneRenderer`, `SceneRendererBuilder`, `scene_renderer::renderer` : construction.
- `add_pass`, `activate_pass`, `deactivate_pass` : passes locales.
- `add_effect`, `effect`, `effect_mut`, `remove_effect`, `set_effect_active` : effets.
- `append_effect`, `insert_before`, `insert_after`, `schedule` : planning.
- `connect`, `resolve_source`, `validate`, `prepare_effects`, `execute` : raccords et rendu.
- `EffectId`, `EffectSource`, `ScheduleAnchor`, `ScheduleError` sont réexportés à
  la racine ; `PassId` et `ScheduledPass` sont dans `api::scene_renderer`.

## Données et propriété

- `RenderData`, `RenderDataId<T>`, `RenderDataError` : insert, get/get_mut,
  require/require_mut, remove.
- `OwnedResource`, `SharedResource`, `ResourceHandle` : new, id, share, clone, Drop.
- `core::Resources` : insert, read_resource, write_resource, stockage par type.
- `core::Input` et `unique_resource!` : utilitaires d'application existants.
- `backend::instances::{Instance, InstanceData, InstanceStorage, InstanceId,
  InstanceError, memory_stats}` : groupes d'instances et statistiques.
- `backend::frame_data::FrameData` reste un ancien conteneur public ; le chemin
  actuel des opérations utilise RenderData. Ne pas bâtir une nouvelle intégration
  sur cet ancien type uniquement parce qu'il est encore accessible.

## Backend GPU

- `context::{Context, ContextId, InitError, init, create, remove, context, device,
  queue, texture_storage, next_id}` : accès global/default et contextes identifiés.
  next_id est un utilitaire d'identité, pas un constructeur de ressource GPU.
- `renderer::{Renderer, RendererInitError, Frame}` : new, resize, begin_frame ;
  execute, size_in_pixels, render_parts, finish. render est déprécié.
- `buffer::{Buffer, BufferBuilder, BufferKind, BindableBufferKind, AllocError,
  BindingError, uniform, storage, storage_rw, storage_vertex, vertex, index,
  write_buffer, delete_buffer}`.
- `bind_group::{Bindable, BindGroupLayoutBuilder, BindGroupBuilder}` et macros
  `try_bind_group!`, `try_bind_group_layout!`, `bind_group!`, `bind_group_layout!`.
- `sampler::{Sampler, SamplerBuilder, filtering, pixel_art}`.
- `load_shader` : chargement WGSL depuis un fichier.

## Cibles et ressources de dessin

- `target::{RenderTarget, RenderTargetId, RenderTargetDescriptor, TargetSize,
  PassTarget, register, get, remove, create_target, resize_window_targets}`.
  create_target produit une cible brute non inscrite ; register l'inscrit.
- `depth::{DepthTarget, FORMAT, opaque_state, depth_from_ground_y}`.
- `binding_set::{TextureBinding, TextureBindingSetId, BindingSetError, register,
  group, update, remove}`.
- `texture::{Texture, TextureBuilder, TextureResource, AtlasRegion, load_texture,
  load_texture_array, register_texture, register_texture_array, register_atlas,
  build_atlas_region, remove}`.
- `shape::{MeshResource, Shape, ShapeInstance, Batch, register, remove, batch,
  batch_into, draw, draw_all}`.
- `sprite::{RenderInstance, Sprite, SpriteInstance, Batch, batch, batch_into,
  draw, draw_all, run_animated}`.
- `geometry::mesh::{Mesh, Vertex}` et `geometry::primitives` pour les primitives.

## Passes, pipelines et phases

- `pass::{PassContext, PassList, PassError, render, compute, execute, execute_one,
  RenderOperation, ComputeOperation, RenderSequence, RenderPassBuilder}` ainsi
  que les builders d'attachments réexportés.
- Macros `define_render_operation!` et `define_compute_operation!`.
- `pipeline::{PipelineContext, PipelineId, DrawPipeline, RenderPipelineBuilder,
  bind, register, get, remove}` et `define_pipeline_context!`, `pipeline_context!`.
- `pipeline::compute::{ComputePipelineBuilder, ComputePipelineContext,
  ComputePipelineId, bind, register, get, remove}` et macros compute associées.
- `phase::{DrawGeometry, DrawCommand, DrawError, RenderPhase}` et
  `sort::{DrawSortKey, SortPolicy}`, également réexportés dans backend.
- `metrics::{RenderMetrics, reset, snapshot}`.
- `create_marker!` déclare des types marqueurs sans état.

## Effets et utilitaires de rendu

- `effects::{RenderEffect, EffectError, EffectInputDesc, EffectInputKind}`.
- `effects::effect::EffectAny` : support de l'accès typé.
- `blur::{BlurEffect, BlurSettings, BlurParams, MAX_BLUR_RADIUS}` et opérations
  horizontal/vertical ; utiliser normalement la PassList possédée par l'effet.
- `composite::{CompositeEffect, CompositionOperation}` : composition et set_source.
- `present::{PresentEffect, PresentOperation}` : présentation de texture.
- Modules `bloom` et `hdr` : emplacements de développement, pas encore des effets
  complets raccordables disponibles.

## Texte, caméra et particules

- `text::{TextDescription, TextLayout, TextResource, TextId, Text, TextInstance,
  TextPlacement, TextSpace, TextClip, TextView, TextError, InitError}`.
- `text::{init, register, remove, set_text, set_layout, prepare, draw_all,
  submitted, discard}` ; styles de police réexportés selon `text/mod.rs`.
- `camera::{Camera2D, Camera2DController}`.
- `particle::{EffectDescription, EffectInstance, EmitterSettings, spawn_effect,
  remove_effect, update_effects, prepare_registered_effects, for_each_prepared_effect}`.
- `particle::modules` contient les modules d'initialisation, simulation et rendu,
  les traits correspondants et les sorties préparées. Voir le guide pour les
  limites actuelles d'extension depuis une autre crate.

Les guides couvrent les usages et leurs responsabilités ; la référence générée
permet de vérifier chaque option de builder avant d'écrire un shader ou un layout.

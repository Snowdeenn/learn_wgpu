# Erreurs, diagnostic et limites actuelles

## Niveaux d'erreurs

- RendererInitError : surface, adaptateur, contexte, format ou texte.
- SceneError : sélection absente/invalide ou erreur de passe.
- ScheduleError : planning, accès typé, connexions et préparation des effets.
- EffectError : entrée inconnue/manquante, cible absente ou incompatible.
- PassError : erreur propagée d'opération, phase, données, binding, cible ou effet.
- RenderDataError : handle/type de données attendu introuvable.
- DrawError : index de commande et raison d'échec.
- InstanceError : ranges, allocations, textures, atlas, meshes ou bindings.
- AllocError / BindingError : allocation de buffer ou résolution de région.
- TextError : ressource, placement, vue, préparation ou cycle de texte.

Toutes les fonctions publiques ne retournent pas un Result : certains builders
et constructeurs actuels utilisent expect/assert ou la validation wgpu. Lire le
chemin choisi, et préférer les variantes try_ lorsqu'elles existent.

Les erreurs d'effets traversant prepare_effects sont actuellement converties
en texte dans ScheduleError. Ne pas dépendre d'un downcast d'EffectError à travers
ce chemin. Les types d'erreurs sont disponibles, mais pas tous les contextes sont
encore structurés en variantes dédiées.

## En cas d'erreur pendant une image

Abandonner la frame et ses commandes ; ne pas appeler finish sur une image
partiellement encodée. Les chemins bas niveau qui possèdent leurs command buffers
doivent également gérer le cycle du texte. Le wrapper Frame possède son nettoyage
de texte et du cache d'instances. Le wrapper Frame remet ces deux cycles à zéro
après destruction de son encodeur, y compris en cas d'abandon. Les opérations
doivent tolérer d'être rappelées la frame suivante.

## Compteurs disponibles

`backend::metrics::reset()` remet les compteurs du thread à zéro. `snapshot()`
retourne batches, draw calls, allocations d'instances et octets uploadés.
`instances::memory_stats()` retourne slots et capacité logique réservée du cache.

```rust
velum::backend::metrics::reset();
renderer.execute(&mut frame, &mut data)?;
let metrics = velum::backend::metrics::snapshot();
println!("{} draws, {} octets", metrics.draw_calls, metrics.uploaded_bytes);
```

Ces compteurs n'incluent pas nécessairement les appels externes, le texte ou egui.
La mémoire du cache n'est pas la totalité de la VRAM, ni l'ensemble des pages,
textures ou copies CPU. Mesurer également les temps CPU de simulation, préparation,
encodage et acquisition. Le CPU autour d'un draw ne mesure pas le temps GPU.

## Symptômes fréquents

**Aucun objet, mais du texte** : vérifier que l'update atteint l'écriture du buffer
caméra, que le pipeline pointe vers ce buffer et que la source n'a pas été détruite.
La préparation du texte et celle des objets suivent des chemins distincts.

**Changement de réglage sans effet** : un paramètre a peut-être été copié dans
l'opération à la construction. Utiliser RenderData ou modifier l'effet concret,
puis laisser prepare transférer les nouvelles valeurs.

**Ressource introuvable juste après sa création** : propriétaire temporaire détruit,
ou seulement un ID conservé à la place du propriétaire.

**Effet désactivé, image figée** : un consommateur local lit la cible propre au
lieu de resolve_source(Output(id)). Les connexions déclarées entre effets suivent
le relais ; les handles fixes d'opérations locales ne le suivent pas tout seuls.

**Image incorrecte après resize** : un ancien Arc ou BindGroup est conservé sans
résolution des révisions, ou une caméra conserve l'ancienne taille.

**Transparents dans le mauvais ordre** : vérifier la politique de phase et le
blending. Le batching respecte l'ordre mais ne devine pas l'intention de mélange.

## Limites à connaître avant intégration

- Les DrawCommand géométriques actuelles reposent sur un format d'instance 2D.
- Le tri 3D par vue et un système général de matériaux ne sont pas fournis.
- Le bloom/HDR/tone mapping complet reste à implémenter.
- Pas d'inférence des dépendances cachées dans les shaders ou passes locales.
- Les IDs de stockages distincts ne bénéficient pas tous d'un contrôle uniforme
  de provenance ; les utiliser exclusivement avec leur propriétaire d'origine.
- Les stockages et certains caches restent globaux ; le multidevice n'est pas
  uniformément raccordé par tous les helpers.
- Libérer les ressources n'oblige pas le cache à rendre ses pages au système.
- Le texte a un cycle global limité, pas un batch indépendant par scène ou vue.
- Les chemins d'assets de plusieurs helpers dépendent du répertoire de travail.
- Les validations d'alias d'effets sont conservatrices et ne remplacent pas une
  analyse complète des sous-ressources GPU.

Ces points décrivent les contraintes actuelles, pas une raison d'introduire un
gestionnaire global supplémentaire dans le code de l'application.

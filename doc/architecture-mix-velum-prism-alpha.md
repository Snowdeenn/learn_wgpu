# Architecture de rendu pour Alpha — combinaison Velum et Prism V2

Document de conception. Il décrit la cible du raccord entre Velum et les concepts retenus de Prism V2. Les types et responsabilités proposés ici devront être validés pendant l'implémentation.

## Objectif

Construire un moteur cohérent dans le style de Velum, avec sa gestion des scènes, son stockage, ses fonctions libres, ses macros, ses passes et ses extensions par traits. Introduire les descriptions de géométrie, de matériaux, d'objets de rendu et de bindings issues de Prism V2 dans ce fonctionnement.

Le client Alpha conserve sa boucle, sa simulation, son registre de ressources applicatives, sa logique de caméra, ses écrans et le déclenchement de ses effets. Le moteur prend en charge les ressources et opérations nécessaires au rendu.

## Décisions validées

| Domaine | Base retenue | Résultat attendu |
| --- | --- | --- |
| Scènes et orchestration | Velum | `Velum`, `Scene`, `SceneRenderer`, activation, données persistantes et planning explicite. |
| Stockage CPU et GPU | Velum | `RenderData`, stockage d'instances, registres GPU, identifiants, `OwnedResource` et `SharedResource`. |
| Registre applicatif par type | Client | Le client fournit le service aujourd'hui couvert par `core::Resources`. |
| Contextes GPU | Velum | Contexte implicite par défaut et contextes identifiés. |
| Buffers | Velum | Allocations typées, sous-allocation, écriture et résolution de régions. |
| Cibles et cycle d'image | Velum | Surface, hors-écran, resize, `Frame`, encodage, soumission et présentation. |
| Descriptions de bindings | Prism V2 | Schémas explicites, groupes, entrées, types de ressources et validation. |
| Construction des bindings | Velum | Macros conservées et raccordées aux schémas communs. |
| Contextes de pipelines | Velum | Contextes typés, macros, stockage et `DrawPipeline`. |
| Matériaux | Prism V2 | Templates, instances, paramètres et ressources liées. |
| Passes graphiques et compute | Velum | `PassContext`, `PassList`, opérations et exécution existantes. |
| Textures et samplers | Velum | Textures individuelles, arrays, atlas, régions et helpers de samplers. |
| Géométrie | Prism V2 | Description des buffers, plages de dessin, indexation et instancing. |
| Objets de rendu | Prism V2 | Géométrie, matériau, instances, bindings, phase et clé de tri. |
| Instances, sprites et formes | Velum | Traits applicatifs, types prêts à l'emploi, préparation et batching. |
| Phases | Velum | Soumission, tri stable et exécution. |
| Texte | Velum | Ressources, placement, préparation et dessin. |
| Particules | Velum | Description, émission, simulation, modules et sorties préparées. |
| Effets | Velum | Propriété, entrées nommées, connexions, préparation, planning et relais. |
| Rapport d'image | Inspiré de Prism V2 | Nouvelle API adaptée aux opérations réellement exécutées par Velum. |
| Caméras | Monde et écran | Deux espaces de rendu accessibles dans le modèle de passes Velum. |

La cible conserve le cycle d'image Velum et les écritures de buffers existantes. Les vues et les uploads différés de `FrameV2` restent hors du périmètre demandé. L'orchestration retenue est le planning Velum avec ses connexions d'effets.

## Style d'API et rôle des macros

Les fonctions libres constituent les points d'entrée prioritaires. L'appelant crée, enregistre, consulte et modifie ses ressources par les modules du backend. Les services internes de stockage et de résolution peuvent être des objets importants, accessibles implicitement par le contexte.

Les objets explicites qui correspondent à une responsabilité publique restent utilisables, notamment `SceneRenderer`, `RenderData`, les effets, les opérations et les phases. Le client doit pouvoir construire un rendu courant avec des fonctions libres et des identifiants.

La construction par macros est une exigence ferme. Chaque capacité actuellement construite par une macro Velum conserve ce chemin de construction après le raccord. Cela concerne notamment les bindings, les contextes graphiques, les contextes compute et les opérations graphiques et compute.

Les macros et les builders explicites alimentent les mêmes descriptions et la même validation. Les nouveaux champs nécessaires aux matériaux ou aux bindings doivent être accessibles depuis les macros concernées. Les variantes `try_` restent le chemin permettant de propager les erreurs.

Les noms et la syntaxe des éventuelles nouvelles macros seront définis avec l'API. Leur expansion doit déléguer à la logique commune pour éviter des règles différentes entre construction explicite et construction par macro.

## Scènes, données et propriété

`Velum` possède les scènes et sélectionne la scène active. Chaque `Scene` conserve son `SceneRenderer` et son `RenderData`, y compris pendant son inactivité. L'application décide quand actualiser ses données et changer de scène.

`RenderData` reste un stockage typé de plusieurs valeurs arbitraires, avec accès par `RenderDataId<T>`. Il peut contenir les propriétaires des ressources propres à une scène, ses instances, ses paramètres et les données lues par ses opérations.

Les géométries, templates, matériaux et jeux de bindings introduits par le raccord utilisent le stockage et le modèle de propriété de Velum. Le moteur expose des identifiants compatibles avec ses registres et retourne des propriétaires lors des créations appropriées.

Proposition de règle de possession à valider dans le détail pendant l'implémentation

- Une géométrie persistante conserve la propriété partagée des allocations de buffers nécessaires à son dessin.
- Un matériau conserve son template et les ressources persistantes auxquelles ses bindings font référence.
- Les ressources de scène sont gardées par `RenderData`, les opérations ou les effets selon leur rôle.
- Un `RenderObject` soumis pour une image référence les ressources persistantes par identifiant. Leurs propriétaires doivent couvrir l'encodage de cette image.
- Un partage entre scènes passe par `SharedResource`. La suppression d'une scène libère ses possessions exclusives et conserve celles encore partagées.

Les références entre ressources doivent former des relations de possession maîtrisées. Les opérations de remplacement et de suppression devront garder un état cohérent, y compris lorsqu'une validation échoue.

## Géométrie, formes et sprites

La description de géométrie reprend le modèle de Prism V2. Elle identifie les vertex buffers et leurs slots, leurs régions, un index buffer facultatif, la plage d'éléments et le décalage de sommet de base. Les dessins indexés, les dessins sans indices et les triangles générés par le shader doivent pouvoir être représentés.

Les références de buffers sont adaptées aux régions allouées par Velum. Les offsets et tailles doivent être respectés pour éviter qu'une géométrie lise une autre allocation de la même page.

Le chemin de formes de Velum conserve ses traits et ses helpers. L'enregistrement d'un mesh prépare une géométrie compatible avec le nouveau modèle. `ShapeInstance` fournit ensuite cette géométrie, les données d'instance et la clé de tri.

Les sprites conservent les textures séparées, arrays, atlas et animations de Velum. Leur préparation résout la géométrie, les UV, la couche de texture et les ressources nécessaires au matériau.

Les utilitaires de tessellation de Prism, dont le nine-patch, sont retenus comme complément de génération de géométrie CPU. Leur intégration doit préserver les formats de sommets attendus par les pipelines qui les consomment.

## Matériaux et contextes de pipelines

`MaterialTemplate` décrit le programme et les états compatibles avec une famille de dessins. Il contient les références aux shaders et à leurs points d'entrée, les layouts de sommets, le schéma de bindings et les états raster pertinents.

`Material` référence un template et fournit les paramètres et ressources propres à une instance de matériau. Plusieurs objets peuvent partager un matériau et plusieurs matériaux peuvent partager un template.

`PipelineContext` conserve le rôle de contexte GPU résolu de Velum. Il fournit le pipeline et les états à lier dans la passe. Les contextes restent typés, enregistrés dans les stockages Velum et accessibles aux commandes hétérogènes par `DrawPipeline`.

Le template constitue la description de référence pour les shaders, layouts et états qu'il porte. La construction par macro doit permettre de produire le contexte compatible avec cette description. Le raccord doit rendre explicite la validation des formats de cible, du blending, de la profondeur et des layouts.

Les opérations personnalisées gardent l'accès au chemin direct de liaison des contextes. Le compute conserve ses contextes, macros, opérations et dispatchs Velum.

## Bindings décrits et construits avec les macros

Les descriptions inspirées de Prism couvrent les groupes, numéros de binding, étapes de shader, uniforms, buffers storage, textures, samplers et textures de stockage. Elles portent les contraintes utiles de taille, dimension, format, accès et offsets dynamiques.

Les macros Velum construisent ces descriptions et les jeux de ressources associés. La déclaration concise et la description explicite doivent produire des layouts équivalents et des erreurs cohérentes.

La résolution assemble les bindings communs au contexte, ceux du matériau et les substitutions de l'objet. Les numéros de groupes et de bindings sont déclarés explicitement par l'application, y compris dans les macros. Chaque ressource peut occuper tout emplacement compatible avec le shader, le layout et les limites du device. Cette liberté couvre les instances, les caméras, les textures et les matériaux. La résolution utilise ces descriptions pour lier les ressources aux emplacements choisis. Les conflits entre contributions sont signalés.

La nouvelle API supprime les conventions de slots imposées par catégorie de ressource. Le buffer d'instances peut notamment être placé dans tout groupe et binding valides. Pour les données d'instance alimentées par un vertex buffer, le slot du buffer et les locations des attributs sont également décrits explicitement. Les macros doivent exposer ces choix. Les points d'entrée historiques conservent leurs conventions pour assurer leur compatibilité.

Les bindings vers des cibles redimensionnables conservent la résolution par identifiant et révision. Un remplacement de texture ou de cible doit rafraîchir les vues et groupes concernés.

L'ajout des caches de layouts, de pipelines et de bind groups est reporté. La première version construit et stocke les ressources avec les mécanismes retenus de Velum. Le cache d'instances déjà utilisé par l'ancienne API reste disponible pour son fonctionnement. Les bindings de cibles continuent de suivre les révisions nécessaires au resize. Le rechargement des shaders et ses règles de révision feront l'objet d'une décision distincte.

## Objets, instances et phases

`RenderObject` reprend les responsabilités du type Prism. Il décrit une géométrie, un matériau, une plage d'instances, des bindings par objet, une phase et une clé de tri. Les identifiants et la représentation de la clé sont adaptés à Velum.

Les traits `Instance`, `ShapeInstance` et `SpriteInstance` restent les entrées ergonomiques des objets applicatifs. Le dessin direct et la soumission dans une phase réutilisent une préparation commune pour conserver la même résolution des ressources et les mêmes contrôles.

L'application fournit la structure de données de ses instances et implémente les traits Velum nécessaires à leur préparation et à leur dessin. Le contrat des traits doit permettre de décrire les données GPU, leur layout et leur liaison aux emplacements choisis par l'application. Cette structure peut ainsi servir aux formes, aux sprites et aux dessins personnalisés.

La nouvelle API doit accepter ces représentations applicatives sans conversion obligatoire vers un `RenderInstance` fixe. Le format `RenderInstance` existant reste utilisable par l'ancienne API et peut être proposé comme format prêt à l'emploi. La correspondance entre données, layout shader et plages d'instances est vérifiée. Les contraintes de transfert mémoire nécessaires sont explicites dans les traits et les descriptions.

`RenderPhase` conserve les politiques de Velum, dont ordre de soumission, ordre explicite et tri de profondeur dans les deux sens. Les égalités conservent l'ordre de soumission. Le batching regroupe les dessins voisins compatibles après tri et respecte les changements de géométrie, matériau, pipeline, texture et bindings.

Les phases peuvent réutiliser leurs vecteurs et allocations de travail. Une erreur ou un abandon d'image doit permettre de préparer correctement l'image suivante.

## Caméra monde et caméra écran

Le moteur doit permettre un rendu monde et un rendu écran dans le planning de passes Velum.

La caméra monde fournit la transformation utilisée par les objets de la scène. Le client peut lui appliquer le suivi du joueur et la secousse. La caméra écran fournit la projection adaptée au HUD et aux éléments d'interface.

Les opérations lient les données de caméra adaptées à leur dessin. Les conventions de coordonnées, de DPI et de redimensionnement doivent être cohérentes avec le texte et les entrées de l'application. Les helpers de caméra Velum restent disponibles.

## Effets, passes et profondeur

Les passes graphiques et compute restent celles de Velum, avec leurs cibles, leurs opérations, leur activation et leur ordre explicite. `RenderSequence` conserve la composition de plusieurs opérations dans une même passe.

Un `RenderEffect` possède ses ressources et sa liste de passes, déclare ses entrées et expose sa sortie. `SceneRenderer` conserve les connexions, prépare les effets et exécute leurs blocs dans le planning.

La désactivation suit le relais déclaré par l'effet. Les consommateurs doivent résoudre la sortie effective lorsque le relais doit être respecté. Le resize doit actualiser cibles et bindings sans reconstruire toute la scène.

Le flou, la composition et la présentation de Velum doivent fonctionner après le raccord. Le document conserve leur niveau d'intégration actuel, notamment les helpers de composition et de présentation. Les chaînes bloom, HDR et tone mapping complètes constituent des évolutions ultérieures.

La profondeur reste portée par les cibles et passes Velum. Les états des templates et contextes doivent correspondre aux attachments. Toute conservation de profondeur entre passes devra être explicitement conçue et validée si elle est ajoutée au comportement actuel.

## Texte et particules

Le texte conserve les ressources, les mises à jour, `TextInstance`, le placement monde ou écran et le cycle de préparation/dessin de Velum. Son intégration au cycle d'image doit garantir un nettoyage correct après soumission et après abandon.

Les particules conservent descriptions, instances, modes d'émission et modules CPU de Velum. Les sorties de formes, sprites et traînées doivent rejoindre le nouveau chemin de géométrie, matériau et objet de rendu.

Les propriétaires de meshes et textures utilisés par les modules doivent rester vivants. Une sortie préparée peut être parcourue pour plusieurs catégories de dessin sans relancer sa préparation.

## Rapport d'image

Une nouvelle API de rapport rassemble les informations disponibles dans Velum. Sa collecte doit suivre le même cycle que la frame et rester utilisable par les fonctions libres et les opérations personnalisées.

Champs proposés, à préciser lors de l'implémentation

| Mesure | Définition attendue |
| --- | --- |
| Passes graphiques et compute | Nombre de passes effectivement encodées, avec distinction des catégories. |
| Draw calls | Nombre de commandes de dessin encodées par les chemins instrumentés. |
| Batches | Nombre de lots préparés et exécutés par les batchers. |
| Instances | Nombre d'instances effectivement soumises par les chemins instrumentés. |
| Uploads | Octets écrits via les chemins de transfert instrumentés de Velum. |
| Allocations | Allocations d'instances et capacités connues des stockages concernés. |
| Préparation et encodage | Temps CPU mesurés sur des périmètres documentés. |
| Diagnostics | Ressources invalides, dessins omis et erreurs de passes ou d'effets avec leur contexte. |

Le texte, les effets intégrés et les opérations du moteur doivent contribuer lorsque la mesure est disponible. Les opérations externes peuvent déclarer leurs compteurs. Chaque mesure indique son périmètre pour éviter de confondre capacité réservée, allocations actives et mémoire physique GPU.

La politique d'erreur par dessin reste à fixer. Une erreur de ressource doit toujours être diagnostiquée. Le choix entre propagation, omission et matériau de secours mérite une décision explicite avant d'adopter ce comportement de Prism.

## Compatibilité et mise en œuvre

L'ensemble de Velum doit fonctionner avec le raccord. Cela couvre la bibliothèque, les exemples, le banc d'essai, le texte, les particules, les effets, les opérations personnalisées, les macros et le rendu direct comme le rendu par phases.

La nouvelle API est ajoutée au-dessus de l'ancienne. L'ancienne API reste présente, accessible et fonctionnelle, avec ses signatures, ses macros et ses comportements existants. Ses utilisateurs peuvent continuer à l'employer pendant l'introduction du nouveau chemin. Les deux API peuvent partager des mécanismes internes tant que cette compatibilité est préservée.

La nouvelle API peut disposer de nouveaux types, traits et points d'entrée. Son nommage ou son espace de modules doit permettre de distinguer les deux chemins. Des exemples dédiés doivent démontrer le nouveau raccord, tandis que les exemples historiques continuent de valider l'ancienne API. La migration du client peut être progressive.

L'implémentation doit être effectuée sur une branche dédiée. Le nom proposé est `design/velum-prism-alpha`, puis une branche d'implémentation pourra être utilisée selon l'organisation du travail. La tentative de création de branche pendant la rédaction a été bloquée par les permissions en lecture seule de `.git`. Ce préalable reste à réaliser dans un environnement qui autorise l'écriture des références Git.

Ordre de travail proposé

1. Définir les descriptions communes, les identifiants et la possession des ressources.
2. Raccorder schémas de bindings, macros, contextes et matériaux.
3. Introduire géométries et objets de rendu, puis adapter instances, formes, sprites et phases.
4. Raccorder les passes, effets, texte et particules à la nouvelle API en préservant leur chemin historique.
5. Ajouter le rapport d'image et vérifier les consommateurs du dépôt.
6. Raccorder progressivement Alpha et comparer les scènes représentatives avec les deux API.

## Critères de validation

- Les constructions actuellement accessibles par macro restent accessibles par macro, y compris avec les nouvelles descriptions.
- Une forme et un sprite standards utilisent le même modèle de dessin qu'un objet à matériau personnalisé.
- Deux scènes partagent des ressources et peuvent être retirées dans les deux ordres sans invalider les ressources encore utilisées.
- Les plages de buffers sous-alloués et les plages d'instances sont respectées.
- Le tri stable et les mélanges alpha et additif reproduisent l'ordre visuel attendu.
- La caméra monde et la caméra écran fonctionnent ensemble avec texte, HUD, DPI et resize.
- Les effets peuvent être paramétrés, activés, désactivés et redimensionnés en conservant les raccords valides.
- Les passes compute et graphiques personnalisées fonctionnent avec les fonctions libres et les macros Velum.
- Les particules conservent leurs sorties de formes, sprites animés et traînées.
- Le hors-écran, l'accès à l'encodeur externe, la soumission et l'abandon d'image restent utilisables.
- Le rapport correspond aux commandes et transferts réellement instrumentés.
- L'ancienne API et ses exemples continuent de fonctionner. Les exemples de la nouvelle API couvrent le raccord et les fonctionnalités de Velum.
- Les groupes et bindings des instances, caméras et textures peuvent être déplacés, avec un shader et un schéma correspondants, sans hypothèse de slots dans le backend.
- Une structure d'instance définie par l'application est dessinée via les traits Velum avec son propre layout GPU.
- La nouvelle API fonctionne avec les constructions explicites prévues, avant toute introduction de caches supplémentaires.

## Points encore ouverts

La sélection des composants est fixée. Les points suivants concernent leur contrat précis et doivent être résolus pendant la conception détaillée.

- Syntaxe finale des macros pour templates, matériaux et bindings spécialisés.
- Règles de possession des dépendances et comportement des suppressions explicites.
- Résolution des contributions de bindings et diagnostic des conflits sur les emplacements choisis par l'application.
- Signature des traits décrivant la structure d'instance fournie par l'application, son layout et sa liaison GPU.
- Politique d'erreur par dessin et éventuel matériau de secours.
- Portée de chaque fonction libre vis-à-vis du contexte par défaut et des contextes identifiés.
- Nommage et organisation de la nouvelle API au-dessus de l'ancienne.
- Gestion des révisions lors des remplacements de ressources et éventuel rechargement des shaders.

## Références locales

- [Architecture Velum](architecture.md)
- [Scènes et planning](scene-renderer.md)
- [Stockage et propriété](ressources.md)
- [Pipelines et macros](pipelines.md)
- [Dessin et instances](dessin.md)
- [Passes](passes.md)
- [Effets](effets.md)
- [Texte, caméra et particules](texte-camera-particules.md)
- [Guide Prism V2](../../prism/docs/V2_GUIDE.md)

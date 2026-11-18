# Architecture et flux

## Les objets à distinguer

`backend::renderer::Renderer` est le propriétaire de la surface de fenêtre et de
sa configuration. Il acquiert les images et applique le redimensionnement.

`Frame` est l'image GPU en cours : encodeur, vue de sortie, texture de surface.
Elle dure une image. `finish()` soumet les commandes et présente la surface.

`SceneRenderer` est une recette persistante : passes locales, effets possédés,
connexions et planning. Il encode dans une frame reçue de l'hôte. Il n'acquiert
pas d'image et ne présente pas lui-même.

`RenderData` est un stockage CPU extensible. Il transporte les données consultées
par les opérations. Ce n'est ni une image ni un command buffer GPU. Il peut
persister entre les frames et contenir les propriétaires de ressources.

`Scene` contient publiquement un `SceneRenderer` et un `RenderData`. Elle n'a pas
de logique métier imposée. `Velum` possède les scènes et sélectionne celle dont
le renderer est exécuté. L'application décide quand changer de scène.

## Flux d'une image

```text
Application : événements, simulation, mise à jour de RenderData
    ↓
Renderer::begin_frame(window)
    ↓
Velum::render(frame)
    ↓
SceneRenderer : validation → résolution des entrées → préparation des effets
    ↓
Planning : passes locales et blocs d'effets dans l'ordre explicite
    ↓
Exécuteur backend : ouvre une passe GPU, appelle son opération
    ↓
Opération : dessin direct ou commandes soumises à une RenderPhase
    ↓
Frame::finish(false) : soumission unique puis présentation
```

Toutes les préparations d'effets ont lieu avant l'encodage du planning. Préparer
un effet ne signifie pas que son producteur a déjà été dessiné : cela prépare
ses bindings et uniforms. Les passes productrices seront ensuite encodées avant
les passes consommatrices.

## Trois ordres différents

1. Le **planning** ordonne les passes et les effets.
2. La **PassList de l'effet** ordonne ses passes internes.
3. La **RenderPhase** trie les commandes de dessin à l'intérieur d'une opération.

Une phase ne crée pas une passe GPU. Une passe ne doit pas être assimilée à un
effet : un effet peut nécessiter plusieurs passes. Aucun tri de commandes ne
déplace un dessin dans une autre phase ou une autre passe.

## Deux graphes indépendants

Le planning dit **quand** exécuter. Les connexions disent **quoi lire**.
Connecter B à la sortie de A n'insère ni A ni B dans le planning. Les programmer
ne les connecte pas non plus. Cette séparation rend les décisions visibles et
évite un réordonnancement automatique inattendu.

## Stockage et propriété

Les stockages GPU restent globaux afin de conserver les fonctions libres du
backend. La durée de vie est contrôlée par `OwnedResource` ou `SharedResource`.
Un identifiant permet de chercher une ressource ; le copier ne la maintient pas
vivante. Les commandes, passes et connexions transportent principalement ces IDs.

Le moteur n'est pas encore un renderer 3D complet. Les commandes géométriques
actuelles utilisent `RenderInstance`, un format 2D. Les phases et les passes
constituent une base réutilisable, pas une promesse de soumission 3D existante.

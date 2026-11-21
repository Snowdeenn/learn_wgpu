# Démarrage et cycle de frame

## Utiliser la bibliothèque

Depuis un projet voisin, une dépendance locale peut être déclarée ainsi :

```toml
[dependencies]
velum = { path = "../learn_wgpu/velum" }
```

Le chemin est à adapter. L'hôte qui manipule directement wgpu, winit ou des
types `new_collection` doit aussi déclarer ces dépendances avec des versions
compatibles avec celles du dépôt. Les types de deux versions de wgpu ne sont
pas interchangeables. Les macros actuelles peuvent également nécessiter `paste`
dans le projet appelant. Copier les versions de `[workspace.dependencies]`
dans le Cargo.toml à la racine de ce dépôt évite
les suppositions de compatibilité.

## Initialisation avec une fenêtre

Dans le callback de création de fenêtre de l'hôte :

```rust
use std::sync::Arc;
use velum::backend::renderer::Renderer;

// window: Arc<winit::window::Window>
let renderer = Renderer::new(Arc::clone(&window))?;
let format = renderer.conf.format;
let resolution = [renderer.conf.width, renderer.conf.height];
```

`Renderer::new` crée la surface, demande l'adaptateur/device, initialise le
contexte par défaut et le texte. Il retourne un `Result`. Ne pas appeler à
nouveau l'initialisation du contexte par défaut : elle ne peut réussir qu'une
fois. Conserver le renderer et la fenêtre entre les événements.

Le format choisi vient des formats annoncés par la surface, avec préférence
pour sRGB. Ne pas imposer arbitrairement un format de présentation non annoncé.

## Première scène

Le module [scene_minimale.rs](examples/scene_minimale.rs) construit une scène
complète : une passe efface une texture dans une couleur visible, un flou lit
cette texture et une passe finale présente le résultat. La couleur constante
rend le flou visuellement discret ; remplacer la source par un dessin permet
d'observer son étalement.

```rust
use velum::Velum;

let mut velum = Velum::new();
// build_scene est fourni dans le module d'exemple.
let (scene, blur_id) = build_scene(format, resolution)?;
let scene_id = velum.add_scene(scene);
velum.activate(scene_id)?;
```

## Redraw

```rust
// Ignorer le rendu pour une fenêtre de taille nulle.
if let Some(mut frame) = renderer.begin_frame(&window)? {
    velum.render(&mut frame)?;
    frame.finish(false);
}
```

`Ok(None)` n'est pas une erreur : l'image peut être indisponible ou nécessiter
une reconfiguration. Une erreur d'encodage doit faire abandonner la frame :
ne pas appeler `finish` après une erreur et soumettre une image partielle.

`finish(true)` demande une mesure d'attente CPU de fin GPU. Cela peut bloquer et
réduire les FPS ; ce n'est pas une mesure de durée GPU par timestamps.

## Resize et boucle d'application

```rust
renderer.resize(new_size);
```

Le renderer reconfigure la surface et redimensionne les cibles `TargetSize::Window`.
Les cibles fixes restent inchangées. La taille zéro est ignorée. L'application
doit aussi actualiser la caméra et les paramètres métier dépendant de la taille.

L'hôte reste responsable de `request_redraw()`, de son horloge, de la simulation
et des événements. Velum n'impose pas sa propre boucle d'événements.

## Intégration directe

Sans gestionnaire de scènes, utiliser simplement `SceneRenderer::execute` avec
un `RenderData`. Sans SceneRenderer, `Frame::execute` peut exécuter une PassList.
Ces chemins sont des alternatives : ne pas exécuter la même liste deux fois.

`Frame::render_parts()` expose l'encodeur et la vue pour une intégration externe
(par exemple egui). Dessiner cet overlay après le contenu prévu, puis appeler
`finish` une seule fois. L'ancien `Frame::render` est déprécié.

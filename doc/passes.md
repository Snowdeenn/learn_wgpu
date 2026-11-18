# Créer des passes et des opérations

## Ce qu'une passe décrit

`PassContext::Render` décrit la cible, le chargement/stockage couleur, la cible de
profondeur optionnelle, une opération et l'activation. `PassContext::Compute`
contient une opération compute et son activation.

Une opération décide du dessin ou du dispatch. La passe choisit la destination.
Cette séparation permet de réutiliser une opération sans lui faire ouvrir la
passe ou présenter la surface.

## Une opération minimale complète

```rust
use velum::RenderData;
use velum::backend::pass::{RenderOperation, PassError};

struct ClearOnly;

impl RenderOperation for ClearOnly {
    fn execute(
        &mut self,
        _pass: &mut wgpu::RenderPass<'_>,
        _data: &mut RenderData,
    ) -> Result<(), PassError> {
        Ok(())
    }
}
```

Ne rien dessiner est valide : l'ouverture de la passe applique quand même son
effacement. Pour créer cette passe :

```rust
let id = renderer.add_pass(velum::backend::pass::render(
    target_id,
    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
    ClearOnly,
    true,
));
```

Le quatrième argument est l'activation initiale. `LoadOp::Clear` initialise la
cible ; `LoadOp::Load` préserve son contenu pour ajouter des dessins. Ne pas lire
un ancien contenu en supposant qu'il a été produit dans cette frame : l'ordre et
l'activation des passes sources restent importants.

`pass::render` utilise `StoreOp::Store` par défaut. Les champs publics du
PassContext permettent une configuration plus fine, avant son insertion.

## Lire des données

Une opération peut conserver un `RenderDataId<T>` et chercher son entrée :

```rust
let objects = data.require_mut(self.objects)?;
```

L'erreur est convertie vers PassError. Conserver l'ID suppose que le RenderData
correspondant et son entrée restent valides. Le paramètre `&mut self` permet
de conserver un cache ou une RenderPhase dans l'opération entre les frames.

## Les macros d'opérations

```rust
velum::define_render_operation! {
    Empty {}
    execute(_this, _pass, _data) {
        Ok(())
    }
}
```

Cette macro déclare `EmptyOperation` et son implémentation. Les champs placés
dans les accolades deviennent les champs de l'opération. Elle n'initialise pas
le GPU ni les ressources à ta place. `define_compute_operation!` suit le même
principe pour `ComputeOperation`. Une implémentation Rust explicite est tout aussi
valide et évite une dépendance à la syntaxe des macros dans une API applicative.

## Plusieurs opérations dans une passe

`RenderSequence` exécute ses opérations dans l'ordre donné, dans la même passe :

```rust
let sequence = velum::backend::pass::RenderSequence::new(vec![
    Box::new(FirstOperation { /* champs propres à l'application */ }),
    Box::new(SecondOperation { /* champs propres à l'application */ }),
]);
```

Chaque opération doit lier son état GPU nécessaire. L'état d'une opération
précédente ne constitue pas un contrat implicite. Les noms ci-dessus sont des
types applicatifs, pas des opérations fournies par Velum.

## Compute

Implémenter `ComputeOperation::execute` avec un `&mut wgpu::ComputePass` et le même
RenderData, puis utiliser `pass::compute(operation, true)`. L'opération appelle
`pipeline::compute::bind`, lie ses ressources et lance `dispatch_workgroups`.
Le renderer peut intercaler compute et render via le même planning local.

## Profondeur

`DepthTarget::new(resolution)` possède une cible de profondeur :

```rust
let depth = velum::backend::depth::DepthTarget::new(resolution);
let pass = velum::backend::pass::render(target_id, load, operation, true)
    .with_depth(depth.id());
data.insert(depth);
```

Le pipeline doit déclarer un état de profondeur compatible, par exemple
`depth::opaque_state()`. L'exécuteur actuel efface la profondeur à 1.0 pour une
passe ayant une cible depth. Ne pas supposer une conservation automatique de la
profondeur entre deux PassContext distincts. Formats, taille et nombre
d'échantillons des pièces jointes sont vérifiés.

## PassList et bas niveau

`PassList::insert` retourne un ID stable ; `ids()` fournit son ordre, `get` et
`get_mut` résolvent un ID. `pass::execute` parcourt la liste ; `execute_one`
encode une seule passe. SceneRenderer réutilise cet exécuteur, sans dupliquer
son code. Choisir le niveau d'entrée nécessaire, et éviter de rejouer les mêmes
passes avec plusieurs points d'entrée.

`RenderPassBuilder` et ses builders d'attachments restent disponibles pour une
intégration directe avec wgpu. Ils configurent une passe réelle, contrairement
à PassContext qui est une description persistante.

# Texte, caméra et particules

## Ressources et instances de texte

`text::register(TextDescription)` crée un texte façonnable possédé. La description
contient contenu, famille, poids, style, taille, hauteur de ligne, largeur maximale,
alignement et wrapping. Les polices sont découvertes via le système avec fallback.

```rust
use velum::backend::text;

let label = text::register(text::TextDescription {
    content: "Bonjour Velum",
    font_size: 20.0,
    line_height: 26.0,
    ..Default::default()
})?;
let mut instance = text::Text::new(label.id(), text::TextSpace::Screen);
instance.position = [20.0, 20.0];
instance.color = [1.0, 1.0, 1.0, 1.0];
```

Conserver `label` tant que l'instance est utilisée. `set_text` et `set_layout`
actualisent la ressource sans changer son ID. Plusieurs instances peuvent utiliser
le même texte à des positions différentes. `TextInstance` permet un type applicatif
personnalisé, avec TextId et TextPlacement.

`TextSpace::Screen` utilise des coordonnées logiques multipliées par le DPI.
`World` utilise la conversion de Camera2D. L'ancre choisit le point d'alignement,
le clip est un rectangle en pixels logiques écran même pour un texte monde.

## Préparer puis dessiner

Avant l'encodage du texte :

```rust
text::prepare(&texts, &text::TextView {
    resolution,
    scale_factor,
    camera: &camera,
})?;
```

Dans une RenderOperation :

```rust
text::draw_all(pass)?;
```

Le contexte de texte est actuellement global. Il accepte une préparation puis
un dessin pour cette frame ; ne pas le traiter comme une collection illimitée
de batches préparables entre plusieurs passes. Un second dessin sans nouveau
cycle peut produire AlreadyDrawn, une préparation supplémentaire FramePending.
Ce chemin n'est pas une DrawCommand de RenderPhase.

`Frame::finish` remet le cycle à zéro après soumission. Si l'on soumet soi-même,
appeler `text::submitted` après la soumission. Pour un abandon bas niveau, détruire
d'abord les encodeurs/command buffers concernés puis appeler `text::discard`.
Ne jamais soumettre ensuite ces anciennes commandes.

## Caméra 2D

`Camera2D::new`, `set_pos`, `set_zoom`, `get_view_proj` et `world_to_screen`
fournissent les conversions. `get_view_proj` doit recevoir une taille non nulle
adaptée à la cible dessinée, pas systématiquement la taille de la surface.

La matrice doit être écrite dans le buffer utilisé par le shader des objets.
Préparer le texte avec cette caméra n'actualise pas automatiquement ce buffer.

`Camera2DController` est un outil optionnel pour l'hôte : transmettre les touches,
événements souris et positions, appeler update avec dt, puis end_frame. Velum ne
capture pas automatiquement les événements winit pour l'application.

## Particules : trois étapes distinctes

```text
update_effects(dt)                 simulation CPU
    ↓
prepare_registered_effects()       extraction des instances de dessin
    ↓
for_each_prepared_effect(...)      visite et dessin des sorties
```

La préparation doit se faire une fois par image utile, pas à chaque visite.
Une visite pour les formes puis une autre pour les sprites réutilise les mêmes
sorties préparées. La visite conserve des verrous : ne pas appeler depuis son
callback une fonction qui reprend le même stockage (spawn/remove/update).

## Construire une description

`EffectDescription` contient EmitterSettings et trois listes de modules :

- Initialisation : durée de vie, position, vitesse, couleur, échelle, rotation.
- Update : âge, forces, intégration, courbes sur la durée de vie, animation.
- Rendu : formes, sprites, sprites animés ou segments de traînée.

Les modules disponibles sont réexportés dans `particle::modules` : consulter
leurs champs et les exemples du banc d'essai avant d'associer des canaux.

```rust
use std::sync::Arc;
use velum::particle::{self, EffectDescription, EmitterSettings};

let description = Arc::new(EffectDescription {
    emitter: EmitterSettings::burst(100, 1.0),
    init_modules: vec![],
    update_modules: vec![],
    render_modules: vec![],
});
let owner = particle::spawn_effect(description, [0.0, 0.0]);
data.insert(owner);
```

Ce fragment montre la propriété et la création, mais ne produit volontairement
aucun dessin : il faut ajouter des modules de durée de vie, de mouvement et de
rendu adaptés. `once`, `continuous`, `burst`, `with_cycles` et `with_period`
configurent l'émission. Une description Arc peut être réutilisée entre instances.

Les modules de rendu référencent des IDs de mesh/texture : leurs propriétaires
doivent rester vivants. Le propriétaire retourné par spawn_effect contrôle
l'inscription de l'effet dans le stockage global.

## Dessiner les sorties

Dans une opération avec un pipeline compatible lié :

```rust
particle::for_each_prepared_effect(|output| {
    // Dans un vrai callback, récupérer et propager l'erreur de dessin.
    let result = velum::backend::shape::draw_all(output.shape(), pass);
    // Le callback ne retourne pas un Result : stocker l'erreur pour la remonter
    // après la visite plutôt que l'ignorer dans une application réelle.
    if let Err(error) = result { eprintln!("{error}"); }
});
```

Pour une opération robuste, conserver le premier résultat d'erreur hors callback
et le retourner après la visite. Les sorties de particules peuvent aussi être
converties en DrawCommand ; elles ne sont pas automatiquement ajoutées aux phases.

`EffectInstance` peut être utilisé localement avec update et prepare plutôt que
via le stockage global. L'extension de modules personnalisés expose encore des
types de vue internes ; ne pas supposer que chaque trait réexporté est déjà une
API d'extension externe complètement stabilisée. Les modules fournis sont le
chemin immédiatement utilisable.

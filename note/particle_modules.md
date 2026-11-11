# Modules de particules

Les modules sont exportés par `crate::particle::modules`. Le moteur travaille en
2D : positions et vitesses ont deux composantes, les angles sont en radians et
les durées en secondes. Les modules s'exécutent dans l'ordre des listes de
`EffectDescription`.

## Modules existants conservés

`InitVelocity`, `RandomLifetime`, `InitColor`, `InitScale`, `UpdateAge`,
`RenderSprite` et `RenderShape` gardent leur implémentation. `InitPos`,
`UpdatePos`, `InitAge`, `InitAccel` et `UpdateScale` sont également conservés.
En particulier, `InitColor` tire toujours chaque composante entre `min` et
`max` : son API n'a pas été remplacée par une palette.

## Initialisation ajoutée

- `InitPosition { shape, region }` : `PositionShape::Point([x, y])`,
  `Circle { center, radius }` ou `Rectangle { center, half_extents }`.
  `SpawnRegion::Volume` échantillonne uniformément l'intérieur ; `Surface`
  échantillonne le contour. La position est locale à l'effet.
- `ConeVelocity { direction, spread, min_speed, max_speed }` : `direction` est
  l'angle central ; `spread` est le demi-angle entre 0 et π. Les vitesses sont
  positives ; des bornes égales donnent une vitesse fixe.
- `RadialVelocity { center, min_speed, max_speed }` : à placer après le module
  de position. Le centre est local à l'effet ; une particule exactement au
  centre reçoit une vitesse nulle.
- `InitRotation { min, max }` et `InitAngularVelocity { min, max }` : orientation
  en radians et vitesse angulaire en radians/seconde. Des bornes égales donnent
  une valeur fixe.

## Mise à jour ajoutée

- `Gravity { acceleration }` ajoute une accélération constante à la vitesse.
- `Drag { coefficient }` applique un amortissement exponentiel ; le coefficient
  est positif ou nul, en secondes inverses.
- `Attractor { point, strength }` applique une accélération d'intensité constante
  vers un point en coordonnées monde, sans singularité au centre.
- `NoiseForce { strength, frequency, seed }` utilise un champ spatial de bruit
  interpolé continu et reproductible. La fréquence est en inverse des unités
  de position, la force est une accélération.
- `IntegratePosition` applique `position += velocity * dt`. Il n'utilise pas
  l'ancien multiplicateur `accel` de `UpdatePos` ; choisir l'un des deux.
- `IntegrateRotation` applique `rotation += angular_velocity * dt`.
- `ColorOverLifetime { start, end }` et `ScaleOverLifetime { start, end }`
  interpolent les valeurs absolues selon `age / lifetime`, borné entre 0 et 1.
  Une durée de vie nulle utilise la valeur finale. Placer `UpdateAge` avant.
- `PlaneCollision { point, normal, restitution, friction }` se place après
  l'intégration. En 2D, le plan est une droite ; la normale pointe vers le côté
  autorisé. La restitution et la friction sont comprises entre 0 et 1. Les
  particules sont traitées comme des points ; la friction retire une fraction
  de la vitesse tangentielle uniquement lors d'un impact.

Ordre courant : `UpdateAge`, forces, `IntegratePosition`, `PlaneCollision`,
`IntegrateRotation`, puis couleur et taille selon la durée de vie.

## Préparation du rendu

- `RenderAnimatedSprite { texture, indices, layers }` attend une texture array.
  La plage `layers` est exclusive à droite et doit désigner des couches valides
  de cette texture. L'animation parcourt cette plage une fois pendant la durée
  de vie et conserve la dernière image en fin de vie. Une plage vide ne produit
  aucune instance. La rotation est prise en compte directement.
- `RenderTrail { texture, indices, max_points, width }` attend une texture
  séparée et les indices d'un rectangle unitaire centré, comme celui de
  `create_rect_vertices()`. Chaque paire de positions successives produit un
  segment orienté de largeur constante. L'historique contient au plus
  `max_points` positions (au moins 2 pour dessiner), enregistrées après les
  mises à jour où la particule se déplace. Il appartient à chaque instance
  d'effet, suit la particule lors de la compaction et est vidé à sa renaissance.
  Il disparaît à la mort de la particule. `prepare()` n'avance pas l'historique.
- `WithRotation(RenderSprite { ... })` et `WithRotation(RenderShape { ... })`
  activent la rotation pour les modules existants sans les modifier. Cet
  adaptateur est réservé aux rendus qui produisent une instance par particule ;
  ne pas envelopper `RenderTrail`.

L'angle est stocké dans le champ explicite `rotation: f32` de `RenderInstance`
et des shaders. Le padding restant contient deux mots réservés ; la taille
de l'instance reste de 64 octets. Les shaders de textures multiplient désormais
la texture par la couleur RGBA de l'instance : `ColorOverLifetime` est donc
visible sur les sprites, y compris son alpha.

Validation : `cargo test --offline` couvre l'échantillonnage, les forces, les
collisions, les courbes, l'animation, la rotation, le recyclage des historiques
et la validation WGSL des trois shaders.

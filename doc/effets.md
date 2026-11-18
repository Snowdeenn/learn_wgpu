# Effets, connexions et préparation

## Définition

Un RenderEffect possède ses ressources et sa PassList. Il décrit des entrées
nommées et une sortie principale. Il ne connaît pas ses voisins ni l'application.
Le renderer conserve le graphe de connexions et le planning explicite.

`EffectInputDesc` annonce nom, caractère obligatoire, catégorie Color ou Depth.
`set_input` reçoit une cible résolue. `output` désigne toujours la sortie propre
de l'effet, même désactivé. `bypass_input` annonce un nom d'entrée à utiliser à
sa place. Les handles d'entrée ne possèdent pas les ressources sources.

## Raccorder deux effets

```rust
let first = renderer.add_effect(first_effect);
let second = renderer.add_effect(second_effect);
renderer.connect(first, "source", EffectSource::Target(source_target))?;
renderer.connect(second, "source", EffectSource::Output(first))?;
renderer.append_effect(first)?;
renderer.append_effect(second)?;
```

Les effets avec d'autres entrées peuvent être raccordés par leur nom. Remettre
une connexion sur le même nom remplace la connexion précédente. Il n'y a pas
actuellement de méthode publique dédiée pour déconnecter une entrée.

Connecter ne programme rien ; programmer ne connecte rien. Le renderer vérifie
les cycles et l'ordre des producteurs déclarés. Il n'effectue aucun tri
topologique automatique. Une erreur de connexion restaure le raccord précédent.

## Cycle exact

```text
execute
  → validation du planning et des dépendances
  → pour chaque effet programmé actif, dans l'ordre :
      résolution des entrées
      vérifications communes
      set_input pour les entrées présentes
      prepare une fois
      vérifications des sorties et conflits déclarés
  → nouvelle vérification de structure
  → encodage du planning, effets désactivés ignorés
```

prepare ne doit pas encoder, soumettre, présenter ou modifier la structure des
passes. Il peut mettre à jour uniforms et bindings. La répétition de set_input
avec la même cible doit être peu coûteuse ; un effet peut mémoriser un dirty flag.

## Désactivation et résultat effectif

```rust
renderer.set_effect_active(first, false)?;
let effective = renderer.resolve_source(EffectSource::Output(second))?;
```

Si le premier flou est désactivé, son consommateur lit son entrée "source".
La résolution suit les relais successifs. Sans relais valide, une sortie utilisée
provoque une erreur, et une modification d'activation refusée conserve l'état
précédent. Désactiver ne libère pas les ressources.

Une passe locale qui garde un RenderTargetId fixe n'est pas automatiquement un
consommateur du graphe. Il faut résoudre la sortie effective et reconnecter son
binding au besoin. Le banc d'essai le fait avec CompositeEffect::set_source.
Ne pas utiliser directement effect.output() si le résultat doit suivre le bypass.

## BlurEffect actuel

```rust
let blur = BlurEffect::new(format, resolution);
let id = renderer.add_effect(blur);
renderer.connect(id, BlurEffect::SOURCE, EffectSource::Target(source_id))?;
renderer.append_effect(id)?;
renderer.effect_mut::<BlurEffect>(id)?.settings.sigma = 2.0;
```

Le flou possède deux cibles et deux passes : source → horizontal → B → vertical
→ A. Sa sortie est A. L'entrée doit être une texture couleur 2D filtrable,
monocouche, non multisamplée, échantillonnable, avec les dimensions de ses cibles.
Elle doit être distincte des deux cibles privées. Le flou ne réécrit pas la source.

Settings expose espacement, rayon, sigma, poids manuels et normalisation. Le rayon
effectif est borné à MAX_BLUR_RADIUS. prepare ne réécrit les uniforms que lorsque
les réglages changent. Le dirty flag d'entrée sert au changement d'ID ; les
révisions des cibles servent au resize. Deux sources distinctes peuvent avoir
la même révision : ce sont deux causes de mise à jour différentes.

Rayon ou espacement zéro peut donner une identité visuelle, mais cela ne supprime
pas automatiquement les deux passes. Les poids non normalisés peuvent aussi
modifier la luminosité même sans étalement.

## Bindings et resize

`binding_set::update(id, entries)` remplace les entrées d'un ensemble en gardant
son ID : les opérations peuvent conserver leurs handles sans être reconstruites.
`binding_set::group` rafraîchit ensuite les vues si les révisions changent.
Un effet qui change de source peut reconstruire son binding dans prepare,
uniquement lorsque le raccord change.

## Validation : portée et limites

La validation actuelle couvre les références programmées, les blocs contigus,
leur ordre interne, les doublons, les cycles déclarés et l'ordre producteur/
consommateur. prepare vérifie les entrées obligatoires, l'usage et la catégorie
des textures, ainsi que l'existence et l'usage render attachment de la sortie.

Le contrôle d'aliasing est conservateur : il interdit qu'un effet écrive dans une
de ses cibles d'entrée déclarées. Il ne réalise pas une analyse générale des
sous-ressources, des shaders ou des buffers internes. Les contraintes de format,
taille, filtrage ou multisampling propres à l'effet restent à valider par celui-ci.
Les dépendances de passes locales, et les bindings cachés d'opérations arbitraires,
ne sont pas déduits automatiquement.

## Effets disponibles et objectifs

BlurEffect implémente le contrat. CompositeEffect fournit une composition plein
écran et set_source ; PresentEffect fournit l'opération de présentation. Ces deux
helpers ne sont pas automatiquement des RenderEffect raccordables. Les fichiers
bloom/hdr du dépôt ne constituent pas encore une chaîne de bloom/HDR complète.

Le bloom visé : image HDR, extraction lumineuse, flou, addition avec l'image
d'origine. Le tone mapping est une étape distincte avant affichage. Ne pas
confondre une texture HDR avec un effet de tone mapping déjà implémenté.

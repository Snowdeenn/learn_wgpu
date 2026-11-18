# Ajouter un effet personnalisé

## Exemple complet

Lire [examples/effet_custom.rs](examples/effet_custom.rs). Ce module définit un
`OverlayEffect` raccordable : il lit une image prémultipliée et la compose sur
une cible privée transparente. Il réutilise CompositeEffect plutôt que d'imposer
un nouveau shader. Le but est de montrer le contrat, la propriété et le raccord
d'une vraie passe, pas de présenter cet effet comme un bloom.

Le module est compilé avec les tests de documentation. L'exécution demande un
backend initialisé et les shaders du dépôt accessibles depuis le répertoire de
travail, car les helpers actuels chargent des chemins relatifs.

## Construction

Le constructeur crée une cible de sortie possédée, le helper de composition et
une PassList. La passe contient des IDs de pipeline et de binding ; le helper
reste stocké dans l'effet pour conserver leurs propriétaires vivants.

Le binding initial pointe vers une cible provisoire, mais la passe ne doit jamais
être exécutée ainsi : prepare refuse l'absence de source, puis reconnecte le
binding à l'entrée validée avant l'encodage. Une API de binding différé pourrait
éviter ce placeholder ; elle n'existe pas dans cet exemple.

## Déclaration et raccord

L'entrée unique s'appelle "source", est obligatoire et de catégorie Color.
set_input rejette les noms inconnus, une cible absente et l'alias avec la sortie.
La connexion ne transfère pas la propriété de la source.

Dans une application :

```rust
let overlay = OverlayEffect::new(format, resolution);
let overlay_id = renderer.add_effect(overlay);
renderer.connect(overlay_id, OverlayEffect::SOURCE, EffectSource::Output(blur_id))?;
renderer.append_effect(overlay_id)?;
```

OverlayEffect est défini par le module d'exemple, pas par la bibliothèque.
Il lit des couleurs prémultipliées car CompositeEffect utilise
PREMULTIPLIED_ALPHA_BLENDING. Pour d'autres conventions, créer un pipeline avec
le blending voulu au lieu de supposer qu'un helper convient à toutes les images.

## Préparation

prepare valide à nouveau la ressource : elle a pu disparaître ou être redimensionnée
depuis set_input. Il reconnecte le binding avec set_source. Ce helper évite de
reconstruire un groupe quand l'ID reste identique ; les révisions gèrent le resize.

Pour ajouter un paramètre (gain, seuil, exposition), définir une structure Rust
Pod compatible WGSL et un buffer uniforme possédé par l'effet. Modifier le shader
et son layout, puis écrire ce buffer dans prepare quand les paramètres changent.
Conserver une ancienne valeur ou une révision permet de limiter les uploads.

[pipeline_custom.rs](examples/pipeline_custom.rs) fournit la partie shader,
uniforme et opération sous forme compilable. Pour l'intégrer à un effet,
conserver ses SolidResources dans l'effet, utiliser sa SolidOperation dans une
passe vers la cible privée et transférer les paramètres dans prepare. Son shader
de couleur ne lit pas d'image ; ajouter les entrées de texture et les layouts
correspondants pour en faire un traitement d'image comme un gain ou un seuil.

## Passes internes

Construire la PassList une seule fois. Une chaîne multipasse ajoute des cibles
privées et insère ses étapes dans l'ordre où elles doivent être exécutées.
La structure doit rester stable après append_effect : ne pas insérer/supprimer
des passes dans prepare. Le renderer ne recalcule pas implicitement le bloc.

Les entrées des opérations peuvent rester des handles stables de bindings.
Mettre à jour un binding est différent de remplacer l'opération entière.

## Sortie et bypass

output renvoie le handle de la cible privée. bypass_input renvoie Some("source")
pour que l'effet désactivé transmette son entrée. Un générateur sans source peut
renvoyer None, mais ses consommateurs ne peuvent alors pas traverser sa
désactivation. Ne pas renvoyer un nom qui n'existe pas dans inputs().

## Paramètres et retrait

L'accès renderer.effect_mut::<OverlayEffect>(id) est rendu possible par EffectAny,
implémenté automatiquement pour les types statiques. Ne pas implémenter à la main
des pointeurs ou casts non vérifiés pour retrouver le type.

L'effet conserve les propriétaires. Le retirer retourne l'objet : ses ressources
ne sont supprimées qu'au Drop de cette valeur. Reconnecter les consommateurs avant
ce retrait. Des commandes déjà encodées ne doivent pas dépendre d'une réutilisation
prématurée d'allocations ; créer/modifier/détruire les ressources sur le thread de
rendu et aux frontières de frame reste le modèle utilisé ici.

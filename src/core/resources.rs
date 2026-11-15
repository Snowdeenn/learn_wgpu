//! Registre et conteneur de ressources pour l'injection de dépendances.
//!
//! Fournit la structure [`Resources`] qui permet le stockage et le partage
//! de structures uniques (singletons) via de la mutabilité interne.

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::sync::Arc;
/// Registre centralisé pour l'injection de dépendances et la gestion des ressources clients.
///
/// Chaque type de ressource est identifié de façon unique par son [`TypeId`].
/// L'accès concurrent au sein d'un même thread est géré dynamiquement par [`RefCell`].
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Resources {
    /// Crée une nouvelle instance vide du registre de ressources.
    #[allow(
        dead_code,
        reason = "API du moteur conservee pour les autres scenes et modules."
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insère une ressource unique dans le conteneur.
    ///
    /// Si une ressource du même type `T` existait déjà, elle sera écrasée.
    pub fn insert<T: 'static>(&mut self, resource: T) {
        self.map
            .insert(TypeId::of::<T>(), RefCell::new(Box::new(resource)));
    }

    /// Fournit un accès en lecture seule à une ressource de type `T`.
    ///
    /// # Panics
    ///
    /// Panique si :
    /// - La ressource de type `T` n'a pas été insérée au préalable dans le conteneur.
    /// - La ressource est actuellement empruntée en écriture ([`write_resource`](Self::write_resource)).
    pub fn read_resource<T: 'static>(&self) -> Ref<'_, T> {
        let cell = self
            .map
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Ressource introuvable : {}", std::any::type_name::<T>()));

        Ref::map(cell.borrow(), |b| {
            b.downcast_ref::<T>()
                .expect("Erreur de downcast de la ressource")
        })
    }

    /// Fournit un accès en écriture (mutable) à une ressource de type `T`.
    ///
    /// # Panics
    ///
    /// Panique si :
    /// - La ressource de type `T` n'a pas été insérée au préalable dans le conteneur.
    /// - La ressource est déjà empruntée (en lecture ou en écriture).
    pub fn write_resource<T: 'static>(&self) -> RefMut<'_, T> {
        let cell = self
            .map
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Ressource introuvable : {}", std::any::type_name::<T>()));

        RefMut::map(cell.borrow_mut(), |b| {
            b.downcast_mut::<T>()
                .expect("Erreur de downcast de la ressource")
        })
    }
}

#[macro_export]
macro_rules! unique_resource {
    ($name:ident, $ty:ty) => {
        pub struct $name(pub $ty);

        impl std::ops::Deref for $name {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

/// Access ID for a resource registered in a global storage.
pub trait ResourceHandle: Copy + 'static {
    /// Removes the resource. Releasing an already removed ID has no effect.
    fn release(self);
}

#[must_use = "Keep the owner alive while its resource is in use"]
pub struct OwnedResource<H: ResourceHandle> {
    handle: H,
}

impl<H: ResourceHandle> std::ops::Deref for OwnedResource<H> {
    type Target = H;
    fn deref(&self) -> &H { &self.handle }
}

impl<H: ResourceHandle> OwnedResource<H> {
    pub const fn new(handle: H) -> Self {
        OwnedResource { handle }
    }

    pub fn share(self) -> SharedResource<H> {
        SharedResource {
            owner: Arc::new(self),
        }
    }

    pub const fn id(&self) -> H {
        self.handle
    }
}

impl<H: ResourceHandle> Drop for OwnedResource<H> {
    fn drop(&mut self) {
        self.handle.release()
    }
}
#[must_use = "Keep a shared owner alive while its resource is in use"]
pub struct SharedResource<H: ResourceHandle> {
    owner: Arc<OwnedResource<H>>,
}

impl<H: ResourceHandle> SharedResource<H> {
    pub fn id(&self) -> H {
        self.owner.id()
    }
}

impl<H: ResourceHandle> Clone for SharedResource<H> {
    fn clone(&self) -> Self {
        SharedResource {
            owner: self.owner.clone(),
        }
    }
}

//! The implementation of specialization
//! for the clone behaviors of [`Relationship`]s and [`RelationshipTarget`]s.
//!
//! Based on the guide by GoldsteinE:
//! https://github.com/GoldsteinE/gh-blog/blob/master/const_deref_specialization/src/lib.md.

use core::marker::PhantomData;

use crate::{
    component::{
        clone_specialization::{impl_specialization_deref_chain, TypeLevelCloneBehaviorVariant},
        Component, ComponentCloneBehavior,
    },
    entity::ComponentCloneCtx,
    hierarchy::Children,
    relationship::{
        Relationship, RelationshipHookMode, RelationshipSourceCollection, RelationshipTarget,
    },
    world::DeferredWorld,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{Reflect, TypePath};

impl_specialization_deref_chain!(
    RelationshipCloneSpecialization,
    RelationshipCloneSpecializationTargetChildren,
    RelationshipCloneSpecializationTargetClone,
    RelationshipCloneSpecializationTargetReflect,
    RelationshipCloneSpecializationClone,
    RelationshipCloneSpecializationReflect,
    RelationshipCloneSpecializationBase,
);

/// 1) `T = Children`
/// 2) `T: RelationshipTarget + Clone`
/// 3) `T: RelationshipTarget + Reflect + TypePath`
/// 4) `T: Relationship + Clone`
/// 5) `T: Relationship + Reflect`
/// 6) Anything else
#[doc(hidden)]
pub struct RelationshipCloneSpecialization<T>(PhantomData<T>);

impl<T> Default for RelationshipCloneSpecialization<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// We know there's no additional data on `Children`,
/// so this handler is an optimization to avoid cloning the entire collection.
#[doc(hidden)]
pub struct RelationshipCloneSpecializationTargetChildren<T>(PhantomData<T>);

impl RelationshipCloneSpecializationTargetChildren<Children> {
    pub fn check(&self) -> RelationshipCloneBehaviorTargetChildren {
        RelationshipCloneBehaviorTargetChildren
    }
}

#[doc(hidden)]
pub struct RelationshipCloneSpecializationTargetClone<T>(PhantomData<T>);

impl<T: RelationshipTarget + Clone> RelationshipCloneSpecializationTargetClone<T> {
    pub fn check(&self) -> RelationshipCloneBehaviorTargetClone<T> {
        RelationshipCloneBehaviorTargetClone(PhantomData)
    }
}

#[doc(hidden)]
pub struct RelationshipCloneSpecializationTargetReflect<T>(PhantomData<T>);

#[cfg(feature = "bevy_reflect")]
impl<T: RelationshipTarget + Reflect + TypePath> RelationshipCloneSpecializationTargetReflect<T> {
    pub fn check(&self) -> RelationshipCloneBehaviorTargetReflect<T> {
        RelationshipCloneBehaviorTargetReflect(PhantomData)
    }
}

#[doc(hidden)]
pub struct RelationshipCloneSpecializationClone<T>(PhantomData<T>);

impl<T: Relationship + Clone> RelationshipCloneSpecializationClone<T> {
    pub fn check(&self) -> RelationshipCloneBehaviorClone<T> {
        RelationshipCloneBehaviorClone(PhantomData)
    }
}

#[doc(hidden)]
pub struct RelationshipCloneSpecializationReflect<T>(PhantomData<T>);

#[cfg(feature = "bevy_reflect")]
impl<T: Relationship + Reflect> RelationshipCloneSpecializationReflect<T> {
    pub fn check(&self) -> RelationshipCloneBehaviorReflect {
        RelationshipCloneBehaviorReflect
    }
}

#[doc(hidden)]
pub struct RelationshipCloneSpecializationBase;

impl RelationshipCloneSpecializationBase {
    pub fn check(&self) -> RelationshipCloneBehaviorBase {
        RelationshipCloneBehaviorBase
    }
}

/// The "clone behavior" for [`RelationshipTarget`]. The [`RelationshipTarget`] will be populated with the proper components
/// when the corresponding [`Relationship`] sources of truth are inserted. Cloning the actual entities
/// in the original [`RelationshipTarget`] would result in duplicates, so we don't do that!
///
/// This will also queue up clones of the relationship sources if the [`EntityCloner`](crate::entity::EntityCloner) is configured
/// to spawn recursively.
pub fn clone_relationship_target<T: RelationshipTarget>(
    component: &T,
    cloned: &mut T,
    context: &mut ComponentCloneCtx,
) {
    if context.linked_cloning() && T::LINKED_SPAWN {
        let collection = cloned.collection_mut_risky();
        for entity in component.iter() {
            collection.add(entity);
            context.queue_entity_clone(entity);
        }
    } else if context.moving() {
        let target = context.target();
        let collection = cloned.collection_mut_risky();
        for entity in component.iter() {
            collection.add(entity);
            context.queue_deferred(move |world, _mapper| {
                // We don't want relationships hooks to run because we are manually constructing the collection here
                _ = DeferredWorld::from(world)
                    .modify_component_with_relationship_hook_mode::<T::Relationship, ()>(
                        entity,
                        RelationshipHookMode::Skip,
                        |r| r.set_risky(target),
                    );
            });
        }
    }
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorTargetChildren;

impl TypeLevelCloneBehaviorVariant for RelationshipCloneBehaviorTargetChildren {
    const CLONE_BEHAVIOR: ComponentCloneBehavior =
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<Children>() {
                let mut cloned = Children::with_capacity(component.len());
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        });
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorTargetClone<C: RelationshipTarget + Clone>(PhantomData<C>);

impl<C: RelationshipTarget + Clone> TypeLevelCloneBehaviorVariant
    for RelationshipCloneBehaviorTargetClone<C>
{
    const CLONE_BEHAVIOR: ComponentCloneBehavior =
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<C>() {
                let mut cloned = component.clone();
                cloned.collection_mut_risky().clear();
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        });
}

#[doc(hidden)]
#[cfg(feature = "bevy_reflect")]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorTargetReflect<C: RelationshipTarget + Reflect + TypePath>(
    PhantomData<C>,
);

#[cfg(feature = "bevy_reflect")]
impl<C: RelationshipTarget + Reflect + TypePath> TypeLevelCloneBehaviorVariant
    for RelationshipCloneBehaviorTargetReflect<C>
{
    const CLONE_BEHAVIOR: ComponentCloneBehavior =
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<C>()
                && let Ok(mut cloned) = component.reflect_clone_and_take::<C>()
            {
                cloned.collection_mut_risky().clear();
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        });
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorClone<C: Component + Clone>(PhantomData<C>);

impl<C: Component + Clone> TypeLevelCloneBehaviorVariant for RelationshipCloneBehaviorClone<C> {
    const CLONE_BEHAVIOR: ComponentCloneBehavior = ComponentCloneBehavior::clone::<C>();
}

#[doc(hidden)]
#[cfg(feature = "bevy_reflect")]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorReflect;

#[cfg(feature = "bevy_reflect")]
impl TypeLevelCloneBehaviorVariant for RelationshipCloneBehaviorReflect {
    const CLONE_BEHAVIOR: ComponentCloneBehavior = ComponentCloneBehavior::reflect();
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct RelationshipCloneBehaviorBase;

impl TypeLevelCloneBehaviorVariant for RelationshipCloneBehaviorBase {
    // Relationships currently must have a `Clone`/`Reflect`-based handler for cloning/moving logic to properly work.
    const CLONE_BEHAVIOR: ComponentCloneBehavior = ComponentCloneBehavior::Ignore;
}

#[cfg(test)]
mod tests {
    use crate::{
        component::{
            clone_specialization::extract_clone_behavior_value, Component, ComponentCloneBehavior,
        },
        entity::Entity,
        hierarchy::Children,
        relationship::clone_specialization::{
            RelationshipCloneBehaviorBase, RelationshipCloneBehaviorClone,
            RelationshipCloneBehaviorTargetChildren, RelationshipCloneBehaviorTargetClone,
            RelationshipCloneSpecialization,
        },
    };
    use alloc::vec::Vec;
    use core::marker::PhantomData;

    #[test]
    fn relationship_clone_specialization() {
        #[derive(Component, PartialEq, Debug)]
        #[relationship(relationship_target = ATarget)]
        struct A(Entity);

        #[derive(Component, PartialEq, Debug)]
        #[relationship_target(relationship = A)]
        struct ATarget(Vec<Entity>);

        #[derive(Component, Clone, PartialEq, Debug)]
        #[relationship(relationship_target = BTarget)]
        struct B(Entity);

        #[derive(Component, Clone, PartialEq, Debug)]
        #[relationship_target(relationship = B)]
        struct BTarget(Vec<Entity>);

        let relationship_behavior_children =
            RelationshipCloneSpecialization::<Children>::default().check();
        let relationship_behavior_a = RelationshipCloneSpecialization::<A>::default().check();
        let relationship_behavior_a_target =
            RelationshipCloneSpecialization::<ATarget>::default().check();
        let relationship_behavior_b = RelationshipCloneSpecialization::<B>::default().check();
        let relationship_behavior_b_target =
            RelationshipCloneSpecialization::<BTarget>::default().check();

        assert_eq!(
            RelationshipCloneBehaviorTargetChildren,
            relationship_behavior_children
        );
        assert_eq!(RelationshipCloneBehaviorBase, relationship_behavior_a);
        assert_eq!(
            RelationshipCloneBehaviorBase,
            relationship_behavior_a_target
        );
        assert_eq!(
            RelationshipCloneBehaviorClone::<B>(PhantomData),
            relationship_behavior_b
        );
        assert_eq!(
            RelationshipCloneBehaviorTargetClone::<BTarget>(PhantomData),
            relationship_behavior_b_target
        );

        match extract_clone_behavior_value(&|| relationship_behavior_children) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!("`relationship_behavior_children` should result in `ComponentCloneBehavior::Custom`"),
        }

        match extract_clone_behavior_value(&|| relationship_behavior_a) {
            ComponentCloneBehavior::Ignore => (),
            _ => panic!(
                "`relationship_behavior_a` should result in `ComponentCloneBehavior::Ignore`"
            ),
        }

        match extract_clone_behavior_value(&|| relationship_behavior_a_target) {
            ComponentCloneBehavior::Ignore => (),
            _ => panic!("`relationship_behavior_a_target` should result in `ComponentCloneBehavior::Ignore`"),
        }

        match extract_clone_behavior_value(&|| relationship_behavior_b) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!(
                "`relationship_behavior_b` should result in `ComponentCloneBehavior::Custom`"
            ),
        }

        match extract_clone_behavior_value(&|| relationship_behavior_b_target) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!("`relationship_behavior_b_target` should result in `ComponentCloneBehavior::Custom`"),
        }
    }

    #[cfg(feature = "bevy_reflect")]
    #[test]
    fn relationship_clone_specialization_reflect() {
        use crate::relationship::clone_specialization::{
            RelationshipCloneBehaviorReflect, RelationshipCloneBehaviorTargetReflect,
        };
        use bevy_reflect::Reflect;

        #[derive(Component, Reflect, PartialEq, Debug)]
        #[relationship(relationship_target = CTarget)]
        struct C(Entity);

        #[derive(Component, Reflect, PartialEq, Debug)]
        #[relationship_target(relationship = C)]
        struct CTarget(Vec<Entity>);

        let relationship_behavior_c = RelationshipCloneSpecialization::<C>::default().check();
        let relationship_behavior_c_target =
            RelationshipCloneSpecialization::<CTarget>::default().check();

        assert_eq!(RelationshipCloneBehaviorReflect, relationship_behavior_c);
        assert_eq!(
            RelationshipCloneBehaviorTargetReflect::<CTarget>(PhantomData),
            relationship_behavior_c_target
        );

        match extract_clone_behavior_value(&|| relationship_behavior_c) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!(
                "`relationship_behavior_c` should result in `ComponentCloneBehavior::Custom`"
            ),
        }

        match extract_clone_behavior_value(&|| relationship_behavior_c_target) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!("`relationship_behavior_c_target` should result in `ComponentCloneBehavior::Custom`"),
        }
    }
}

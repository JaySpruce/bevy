//! The implementation of specialization
//! for the clone behaviors of [`Component`]s.
//!
//! Based on the guide by GoldsteinE:
//! https://github.com/GoldsteinE/gh-blog/blob/master/const_deref_specialization/src/lib.md.

use core::marker::PhantomData;

use crate::component::{Component, ComponentCloneBehavior};

#[doc(hidden)]
pub trait TypeLevelCloneBehaviorVariant {
    const CLONE_BEHAVIOR: ComponentCloneBehavior;
}

#[doc(hidden)]
pub const fn extract_clone_behavior_value<B: TypeLevelCloneBehaviorVariant>(
    _: &impl FnOnce() -> B,
) -> ComponentCloneBehavior {
    B::CLONE_BEHAVIOR
}

#[doc(hidden)]
macro_rules! impl_specialization_deref_chain {
    ($first:ident, $second:ident, $($rest:ident),+ $(,)?) => {
        impl<T> core::ops::Deref for $first<T> {
            type Target = $second<T>;
            fn deref(&self) -> &Self::Target {
                &$second(PhantomData)
            }
        }
        impl_specialization_deref_chain!($second, $($rest),+);
    };
    ($penult:ident, $last:ident $(,)?) => {
        impl<T> core::ops::Deref for $penult<T> {
            type Target = $last;
            fn deref(&self) -> &Self::Target {
                &$last
            }
        }
    };
}

pub(crate) use impl_specialization_deref_chain;

impl_specialization_deref_chain!(
    ComponentCloneSpecialization,
    ComponentCloneSpecializationClone,
    ComponentCloneSpecializationBase,
);

#[doc(hidden)]
pub struct ComponentCloneSpecialization<T>(PhantomData<T>);

impl<T> Default for ComponentCloneSpecialization<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[doc(hidden)]
pub struct ComponentCloneSpecializationClone<T>(PhantomData<T>);

impl<T: Clone + Component> ComponentCloneSpecializationClone<T> {
    pub fn check(&self) -> ComponentCloneBehaviorClone<T> {
        ComponentCloneBehaviorClone(PhantomData)
    }
}

#[doc(hidden)]
pub struct ComponentCloneSpecializationBase;

impl ComponentCloneSpecializationBase {
    pub fn check(&self) -> ComponentCloneBehaviorDefault {
        ComponentCloneBehaviorDefault
    }
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct ComponentCloneBehaviorClone<C: Component + Clone>(PhantomData<C>);

impl<C: Component + Clone> TypeLevelCloneBehaviorVariant for ComponentCloneBehaviorClone<C> {
    const CLONE_BEHAVIOR: ComponentCloneBehavior = ComponentCloneBehavior::clone::<C>();
}

#[doc(hidden)]
#[derive(PartialEq, Debug)]
pub struct ComponentCloneBehaviorDefault;

impl TypeLevelCloneBehaviorVariant for ComponentCloneBehaviorDefault {
    const CLONE_BEHAVIOR: ComponentCloneBehavior = ComponentCloneBehavior::Default;
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use crate::component::{
        clone_specialization::{
            extract_clone_behavior_value, ComponentCloneBehaviorClone,
            ComponentCloneBehaviorDefault, ComponentCloneSpecialization,
        },
        Component, ComponentCloneBehavior,
    };

    #[test]
    fn component_clone_specialization() {
        #[derive(Component, PartialEq, Debug)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B;

        let component_behavior_a = ComponentCloneSpecialization::<A>::default().check();
        let component_behavior_b = ComponentCloneSpecialization::<B>::default().check();

        assert_eq!(ComponentCloneBehaviorDefault, component_behavior_a);
        assert_eq!(
            ComponentCloneBehaviorClone::<B>(PhantomData),
            component_behavior_b
        );

        match extract_clone_behavior_value(&|| component_behavior_a) {
            ComponentCloneBehavior::Default => (),
            _ => {
                panic!("`component_behavior_a` should result in `ComponentCloneBehavior::Default`")
            }
        }

        match extract_clone_behavior_value(&|| component_behavior_b) {
            ComponentCloneBehavior::Custom(_) => (),
            _ => panic!("`component_behavior_b` should result in `ComponentCloneBehavior::Custom`"),
        }
    }
}

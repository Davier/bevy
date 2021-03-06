mod access;
mod fetch;
mod filter;
mod iter;
mod state;

pub use access::*;
pub use fetch::*;
pub use filter::*;
pub use iter::*;
pub use state::*;

#[cfg(test)]
mod tests {
    use bevy_reflect::{reflect_trait, Reflect, TypeRegistryArc};

    use crate::{
        component::{ComponentDescriptor, StorageType},
        prelude::Entity,
        reflect::ReflectComponent,
        world::World,
    };

    use super::Trait;

    #[derive(Debug, Eq, PartialEq, Reflect, Default)]
    #[reflect(TestTrait, Component)]
    struct A(usize);
    #[derive(Debug, Eq, PartialEq, Reflect, Default)]
    #[reflect(TestTrait, Component)]
    struct B(usize);

    #[test]
    fn query() {
        let mut world = World::new();
        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }
        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[test]
    fn multi_storage_query() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<A>(StorageType::SparseSet))
            .unwrap();

        world.spawn().insert_bundle((A(1), B(2)));
        world.spawn().insert_bundle((A(2),));

        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }

        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[reflect_trait]
    pub trait TestTrait: std::fmt::Debug {
        fn print(&self) {
            dbg!(self);
        }
    }

    impl TestTrait for A {}

    impl TestTrait for B {}

    #[test]
    fn trait_query() {
        let mut world = World::new();
        let type_registry = world.get_resource_or_insert_with(|| TypeRegistryArc::default());
        type_registry.write().register::<A>();
        type_registry.write().register::<B>();
        world.spawn().insert(A(1));
        world.spawn().insert(B(2));
        world.spawn().insert_bundle((A(3), B(4)));

        for (entity, test_trait) in world
            .query::<(Entity, Trait<ReflectTestTrait>)>()
            .iter(&world)
        {
            dbg!(entity);
            for (reflect_trait, reflect_value) in test_trait.iter() {
                let value = reflect_trait.get(*reflect_value).unwrap();
                value.print();
            }
        }
    }
}

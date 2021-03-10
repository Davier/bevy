use crate::{
    system::{check_system_counter_impl, BoxedSystem, IntoSystem, System, SystemId},
    world::World,
};
use std::borrow::Cow;

pub trait ExclusiveSystem: Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;

    fn id(&self) -> SystemId;

    fn run(&mut self, world: &mut World);

    fn initialize(&mut self, world: &mut World);

    fn check_system_counter(&mut self, global_system_counter: u32);
}

pub struct ExclusiveSystemFn {
    func: Box<dyn FnMut(&mut World) + Send + Sync + 'static>,
    name: Cow<'static, str>,
    id: SystemId,
    system_counter: u32,
}

impl ExclusiveSystem for ExclusiveSystemFn {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn run(&mut self, world: &mut World) {
        // The previous value is saved in case this exclusive system is run by another exclusive system
        let saved_counter = world.exclusive_system_counter;
        world.exclusive_system_counter = self.system_counter;

        (self.func)(world);

        let global_system_counter = world.global_system_counter.get_mut();
        self.system_counter = *global_system_counter;
        *global_system_counter += 1;

        world.exclusive_system_counter = saved_counter;
    }

    fn initialize(&mut self, _: &mut World) {}

    fn check_system_counter(&mut self, global_system_counter: u32) {
        check_system_counter_impl(
            &mut self.system_counter,
            global_system_counter,
            self.name.as_ref(),
        );
    }
}

pub trait IntoExclusiveSystem<Params, SystemType> {
    fn exclusive_system(self) -> SystemType;
}

impl<F> IntoExclusiveSystem<&mut World, ExclusiveSystemFn> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn exclusive_system(self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(self),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            // The value of -1 means that everything should be detected as added/changed
            system_counter: u32::MAX,
        }
    }
}

pub struct ExclusiveSystemCoerced {
    system: BoxedSystem<(), ()>,
}

impl ExclusiveSystem for ExclusiveSystemCoerced {
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn id(&self) -> SystemId {
        self.system.id()
    }

    fn run(&mut self, world: &mut World) {
        self.system.run((), world);
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn check_system_counter(&mut self, global_system_counter: u32) {
        self.system.check_system_counter(global_system_counter);
    }
}

impl<S, Params, SystemType> IntoExclusiveSystem<(Params, SystemType), ExclusiveSystemCoerced> for S
where
    S: IntoSystem<Params, SystemType>,
    SystemType: System<In = (), Out = ()>,
{
    fn exclusive_system(self) -> ExclusiveSystemCoerced {
        ExclusiveSystemCoerced {
            system: Box::new(self.system()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        query::With,
        schedule::{Stage, SystemStage},
        system::{Commands, IntoExclusiveSystem, IntoSystem, Query, ResMut},
        world::World,
    };
    #[test]
    fn parallel_with_commands_as_exclusive() {
        let mut world = World::new();

        fn removal(
            mut commands: Commands,
            query: Query<Entity, With<f32>>,
            mut counter: ResMut<usize>,
        ) {
            for entity in query.iter() {
                *counter += 1;
                commands.remove::<f32>(entity);
            }
        }

        let mut stage = SystemStage::parallel().with_system(removal.system());
        world.spawn().insert(0.0f32);
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        let mut stage = SystemStage::parallel().with_system(removal.exclusive_system());
        world.spawn().insert(0.0f32);
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }
}

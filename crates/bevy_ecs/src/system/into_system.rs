use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::{Access, FilteredAccessSet},
    system::{
        check_system_counter_impl, System, SystemId, SystemParam, SystemParamFetch,
        SystemParamState,
    },
    world::World,
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

pub struct SystemState {
    pub(crate) id: SystemId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    // NOTE: this must be kept private. making a SystemState non-send is irreversible to prevent SystemParams from overriding each other
    is_send: bool,
    pub(crate) system_counter: u32,
}

impl SystemState {
    fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            is_send: true,
            id: SystemId::new(),
            // The value of -1 means that everything should be detected as added/changed
            system_counter: u32::MAX,
        }
    }

    #[inline]
    pub fn is_send(&self) -> bool {
        self.is_send
    }

    #[inline]
    pub fn set_non_send(&mut self) {
        self.is_send = false;
    }
}

pub trait IntoSystem<Params, SystemType: System> {
    fn system(self) -> SystemType;
}

// Systems implicitly implement IntoSystem
impl<Sys: System> IntoSystem<(), Sys> for Sys {
    fn system(self) -> Sys {
        self
    }
}

pub struct In<In>(pub In);
pub struct InputMarker;

pub struct FunctionSystem<In, Out, Param, Marker, F>
where
    Param: SystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_state: SystemState,
    config: Option<<Param::Fetch as SystemParamState>::Config>,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> (In, Out, Marker)>,
}

impl<In, Out, Param: SystemParam, Marker, F> FunctionSystem<In, Out, Param, Marker, F> {
    pub fn config(
        mut self,
        f: impl FnOnce(&mut <Param::Fetch as SystemParamState>::Config),
    ) -> Self {
        f(self.config.as_mut().unwrap());
        self
    }
}

impl<In, Out, Param, Marker, F> IntoSystem<Param, FunctionSystem<In, Out, Param, Marker, F>> for F
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    fn system(self) -> FunctionSystem<In, Out, Param, Marker, F> {
        FunctionSystem {
            func: self,
            param_state: None,
            config: Some(Default::default()),
            system_state: SystemState::new::<F>(),
            marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> System for FunctionSystem<In, Out, Param, Marker, F>
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type In = In;
    type Out = Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.system_state.name.clone()
    }

    #[inline]
    fn id(&self) -> SystemId {
        self.system_state.id
    }

    #[inline]
    fn new_archetype(&mut self, archetype: &Archetype) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.new_archetype(archetype, &mut self.system_state);
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        &self.system_state.component_access_set.combined_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_state.archetype_component_access
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.system_state.is_send
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let global_system_counter = world.increment_global_system_counter();
        let out = self.func.run(
            input,
            self.param_state.as_mut().unwrap(),
            &self.system_state,
            world,
            global_system_counter,
        );
        self.system_state.system_counter = global_system_counter;
        out
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.apply(world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(<Param::Fetch as SystemParamState>::init(
            world,
            &mut self.system_state,
            self.config.take().unwrap(),
        ));
    }

    #[inline]
    fn check_system_counter(&mut self, global_system_counter: u32) {
        check_system_counter_impl(
            &mut self.system_state.system_counter,
            global_system_counter,
            self.system_state.name.as_ref(),
        );
    }
}

pub trait SystemParamFunction<In, Out, Param: SystemParam, Marker>: Send + Sync + 'static {
    fn run(
        &mut self,
        input: In,
        state: &mut Param::Fetch,
        system_state: &SystemState,
        world: &World,
        global_system_counter: u32,
    ) -> Out;
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($param: SystemParam),*> SystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
            Func:
                FnMut($($param),*) -> Out +
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out + Send + Sync + 'static, Out: 'static
        {
            #[inline]
            fn run(&mut self, _input: (), state: &mut <($($param,)*) as SystemParam>::Fetch, system_state: &SystemState, world: &World, global_system_counter: u32) -> Out {
                unsafe {
                    let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(state, system_state, world, global_system_counter);
                    self($($param),*)
                }
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($param: SystemParam),*> SystemParamFunction<Input, Out, ($($param,)*), InputMarker> for Func
        where
            Func:
                FnMut(In<Input>, $($param),*) -> Out +
                FnMut(In<Input>, $(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out + Send + Sync + 'static, Out: 'static
        {
            #[inline]
            fn run(&mut self, input: Input, state: &mut <($($param,)*) as SystemParam>::Fetch, system_state: &SystemState, world: &World, global_system_counter: u32) -> Out {
                unsafe {
                    let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(state, system_state, world, global_system_counter);
                    self(In(input), $($param),*)
                }
            }
        }
    };
}

all_tuples!(impl_system_function, 0, 12, F);

use crate::{
    component::Component,
    entity::Entity,
    query::{
        Fetch, FilterFetch, QueryEntityError, QueryIter, QueryState, ReadOnlyFetch, WorldQuery,
    },
    system::QueryComponentError,
    world::{Mut, World},
};
use bevy_tasks::TaskPool;
use std::any::TypeId;

/// [DirectQuery] is a wrapper for [QueryState] that owns it. It is used for one-time direct queries
/// on the [World].
pub struct DirectQuery<'w, Q: WorldQuery, F: WorldQuery = ()>
where
    F::Fetch: FilterFetch,
{
    pub(crate) world: &'w World,
    pub(crate) state: QueryState<Q, F>,
    pub(crate) system_counter: u32,
    pub(crate) global_system_counter: u32,
}

impl<'w, Q: WorldQuery, F: WorldQuery> DirectQuery<'w, Q, F>
where
    F::Fetch: FilterFetch,
{
    /// # Safety
    /// This will create a Query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the Queries have unique mutable access.
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w World,
        state: QueryState<Q, F>,
        system_counter: u32,
        global_system_counter: u32,
    ) -> Self {
        Self {
            world,
            state,
            system_counter,
            global_system_counter,
        }
    }

    /// Iterates over the query results. This can only be called for read-only queries
    #[inline]
    pub fn iter(&self) -> QueryIter<'w, '_, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.iter_unchecked_manual(
                self.world,
                self.system_counter,
                self.global_system_counter,
            )
        }
    }

    /// Iterates over the query results
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'w, '_, Q, F> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.iter_unchecked_manual(
                self.world,
                self.system_counter,
                self.global_system_counter,
            )
        }
    }

    /// Iterates over the query results
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, '_, Q, F> {
        // SEMI-SAFE: system runs without conflicts with other systems. same-system queries have
        // runtime borrow checks when they conflict
        self.state.iter_unchecked_manual(
            self.world,
            self.system_counter,
            self.global_system_counter,
        )
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal iterator. This can only be called for read-only queries
    #[inline]
    pub fn for_each(&self, f: impl FnMut(<Q::Fetch as Fetch<'w>>::Item))
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                self.world,
                f,
                self.system_counter,
                self.global_system_counter,
            )
        };
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal iterator.
    #[inline]
    pub fn for_each_mut(&self, f: impl FnMut(<Q::Fetch as Fetch<'w>>::Item)) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                self.world,
                f,
                self.system_counter,
                self.global_system_counter,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    #[inline]
    pub fn par_for_each(
        &self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                self.world,
                task_pool,
                batch_size,
                f,
                self.system_counter,
                self.global_system_counter,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    #[inline]
    pub fn par_for_each_mut(
        &mut self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                self.world,
                task_pool,
                batch_size,
                f,
                self.system_counter,
                self.global_system_counter,
            )
        };
    }

    /// Gets the query result for the given `entity`
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.system_counter,
                self.global_system_counter,
            )
        }
    }

    /// Gets the query result for the given `entity`
    #[inline]
    pub fn get_mut(
        &mut self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // // SAFE: system runs without conflicts with other systems. same-system queries have
        // runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.system_counter,
                self.global_system_counter,
            )
        }
    }

    /// Gets the query result for the given `entity`
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SEMI-SAFE: system runs without conflicts with other systems. same-system queries have
        // runtime borrow checks when they conflict
        self.state.get_unchecked_manual(
            self.world,
            entity,
            self.system_counter,
            self.global_system_counter,
        )
    }

    /// Gets a reference to the entity's component of the given type. This will fail if the entity
    /// does not have the given component type or if the given component type does not match
    /// this query.
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryComponentError> {
        let world = self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_read(archetype_component)
        {
            entity_ref
                .get::<T>()
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingReadAccess)
        }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the
    /// entity does not have the given component type or if the given component type does not
    /// match this query.
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFE: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the
    /// entity does not have the given component type or the component does not match the query.
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_component_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        let world = self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_write(archetype_component)
        {
            entity_ref
                .get_unchecked_mut::<T>()
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingWriteAccess)
        }
    }
}

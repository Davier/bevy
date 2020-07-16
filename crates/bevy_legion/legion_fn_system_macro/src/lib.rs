extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

fn tuple(idents: &[Ident]) -> proc_macro2::TokenStream {
    if idents.len() == 0 {
        quote! { ()}
    } else if idents.len() == 1 {
        quote! { #(#idents),* }
    } else {
        quote! { (#(#idents),*) }
    }
}

fn get_idents(fmt_string: fn(usize) -> String, count: usize) -> Vec<Ident> {
    (0..=count)
        .map(|i| Ident::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Ident>>()
}

#[proc_macro]
pub fn impl_fn_query_systems(_input: TokenStream) -> TokenStream {
    let max_resources = 8;
    let max_queries = 4;

    let resources = get_idents(|i| format!("R{}", i), max_resources);
    let resource_vars = get_idents(|i| format!("r{}", i), max_resources);
    let views = get_idents(|i| format!("V{}", i), max_queries);
    let query_vars = get_idents(|i| format!("q{}", i), max_queries);

    let mut tokens = TokenStream::new();

    let command_buffer = vec![Ident::new("CommandBuffer", Span::call_site())];
    let command_buffer_var = vec![Ident::new("_command_buffer", Span::call_site())];
    let subworld = vec![Ident::new("SubWorld", Span::call_site())];
    let subworld_var = vec![Ident::new("_world", Span::call_site())];
    for resource_count in 0..=max_resources {
        let resource = &resources[0..resource_count];
        let resource_var = &resource_vars[0..resource_count];

        let resource_tuple = tuple(resource);
        let resource_var_tuple = tuple(resource_var);

        let resource_permissions = if resource_count == 0 {
            quote! { Permissions::new() }
        } else {
            quote! {
                <#resource_tuple as ResourceSet>::requires_permissions()
            }
        };

        for query_count in 0..=max_queries {
            let view = &views[0..query_count];
            let query_var = &query_vars[0..query_count];

            let query_var_tuple = tuple(query_var);
            let subworld = &subworld[0..query_count.min(1)];
            let subworld_var = &subworld_var[0..query_count.min(1)];

            let component_permissions = quote! {{
                let mut permissions = Permissions::new();
                #(permissions.add(#view::requires_permissions());)*
                permissions
            }};

            for command_buffer_index in 0..2 {
                let command_buffer = &command_buffer[0..command_buffer_index];
                let command_buffer_var = &command_buffer_var[0..command_buffer_index];

                tokens.extend(TokenStream::from(quote! {
                    impl<Func,
                        #(#resource: ResourceSet<PreparedResources = #resource> + 'static + Clone,)*
                        #(#view: for<'b> View<'b> + DefaultFilter + ViewElement),*
                    > IntoSystem<(#(#command_buffer)*), (#(#resource,)*), (#(#view,)*)> for Func
                    where
                        Func: FnMut(#(#resource,)* #(&mut #command_buffer,)* #(&mut #subworld,)* #(&mut Query<#view, <#view as DefaultFilter>::Filter>),*) + Send + Sync + 'static,
                        #(<#view as DefaultFilter>::Filter: Sync),*
                    {
                        fn system_id(mut self, id: SystemId) -> Box<dyn Schedulable> {
                            let resource_permissions: Permissions<ResourceTypeId> = #resource_permissions;
                            let component_permissions: Permissions<ComponentTypeId> = #component_permissions;
                            let run_fn = FuncSystemFnWrapper(
                                move |_command_buffer,
                                    _world,
                                    _resources: #resource_tuple,
                                    _queries: &mut (#(Query<#view, <#view as DefaultFilter>::Filter>),*)
                                | {
                                    let #resource_var_tuple = _resources;
                                    let #query_var_tuple = _queries;
                                    self(#(#resource_var,)*#(#command_buffer_var,)*#(#subworld_var,)*#(#query_var),*)
                                },
                                PhantomData,
                            );

                            Box::new(FuncSystem {
                                name: id,
                                queries: AtomicRefCell::new((#(<#view>::query()),*)),
                                access: SystemAccess {
                                    resources: resource_permissions,
                                    components: component_permissions,
                                    tags: Permissions::default(),
                                },
                                // TODO: by setting to ALL, we're missing out on legion's ability to parallelize archetypes
                                // archetypes: ArchetypeAccess::Some(BitSet::default()),
                                archetypes: ArchetypeAccess::All,
                                _resources: PhantomData::<#resource_tuple>,
                                command_buffer: FxHashMap::default(),
                                run_fn: AtomicRefCell::new(run_fn),
                            })
                        }

                        fn system_named(self, name: &'static str) -> Box<dyn Schedulable> {
                            self.system_id(name.into())
                        }

                        fn system(self) -> Box<dyn Schedulable> {
                            self.system_id(std::any::type_name::<Self>().to_string().into())
                        }
                    }
                }));
            }
        }
    }
    tokens
}
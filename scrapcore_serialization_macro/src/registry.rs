use std::ops::Deref;

use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, ItemStruct, Type, Visibility};

use crate::error::{tokens, MacroError};
use crate::registry::parser::parse_struct_defs;
use crate::{MOD_ERRORS, MOD_REGISTRY};

mod parser;

pub fn registry_impl(
    attr: proc_macro::TokenStream,
    item_struct: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(item_struct);
    tokens(registry_impl_inner(attr.into(), item_struct)).into()
}

#[derive(Debug)]
struct ModelKind {
    span: Span,
    /// Model name in snake_case, for usage as field name
    field_name: Ident,
    /// Model name in PascalCase, for usage as enum variant name
    variant_name: Ident,
    /// Name of the ID type for this model (only applies to collections)
    id_name: Ident,
    ty: Type,
    ty_serialized: Type,
}

#[derive(Debug)]
struct AssetKind {
    span: Span,
    /// Model name in snake_case, for usage as field name
    field_name: Ident,
    /// Model name in PascalCase, for usage as enum variant name
    variant_name: Ident,
    ty: Type,
}

#[derive(Debug, Default)]
struct ModelSet(Vec<ModelKind>);

impl ModelSet {
    fn variants(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.iter().map(
            |ModelKind {
                 variant_name,
                 ty,
                 span,
                 ..
             }| { quote_spanned!(*span=>#variant_name(#ty)) },
        )
    }
}

impl Deref for ModelSet {
    type Target = Vec<ModelKind>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct RegistryDefinitions {
    kind_name: Ident,
    assets_kind_name: Ident,
    model_name: Ident,
    serialized_model_name: Ident,
    registry_name: Ident,
    partial_registry_name: Ident,
    visibility: Visibility,
    schema: bool,

    error: Type,

    singletons: ModelSet,
    collections: ModelSet,
    assets: Vec<AssetKind>,
}

pub(crate) fn registry_impl_inner(
    attr: TokenStream,
    mut item_struct: ItemStruct,
) -> Result<TokenStream, MacroError> {
    let definitions = parse_struct_defs(attr, &mut item_struct)?;

    let model = definitions.model();
    let model_serialized = definitions.model_serialized();
    let kind = definitions.kind();
    let kind_providers = definitions.kind_providers();
    let registry = definitions.registry();
    let partial_registry = definitions.partial_registry();
    let impls = definitions.registry_impls();
    let finalize = definitions.partial_finalize();
    let insert_impl = definitions.insert_impl();
    let item_ids = definitions.item_ids();

    Ok(quote! {
        #model
        #item_ids
        #model_serialized
        #kind
        #kind_providers
        #registry
        #partial_registry
        #impls
        #finalize
        #insert_impl
    })
}

impl RegistryDefinitions {
    /// Definitions for serialized model enum
    fn model_serialized(&self) -> TokenStream {
        let reg = MOD_REGISTRY.deref();
        let collections =
            self.collections.iter().map(
                |ModelKind {
                     variant_name,
                     ty_serialized,
                     span,
                     ..
                 }| {
                    quote_spanned!(*span=>#variant_name(#reg::entry::RegistryEntrySerialized<#ty_serialized>))
                },
            );
        let singletons = self.singletons.iter().map(
            |ModelKind {
                 variant_name,
                 ty_serialized,
                 span,
                 ..
             }| { quote_spanned!(*span=>#variant_name(#ty_serialized)) },
        );
        let visibility = &self.visibility;
        let serialized_model_name = &self.serialized_model_name;
        let schema_derive = self.schema.then(|| quote!(#[derive(schemars::JsonSchema)]));
        let model_name_str = self.model_name.to_string();
        let model_enum = quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #schema_derive
            #[serde(tag = "type")]
            #[serde(rename_all = "PascalCase")]
            #[serde(rename = #model_name_str)]
            #visibility enum #serialized_model_name {
                #(#singletons,)*
                #(#collections,)*
            }
        };

        model_enum
    }

    /// Definitions for the model enum
    fn model(&self) -> TokenStream {
        let singletons = self.singletons.variants();
        let registries = self.collections.variants();
        let model_name = &self.model_name;
        let visibility = &self.visibility;

        let model_enum = quote! {
            #[derive(Debug)]
            #visibility enum #model_name {
                #(#singletons,)*
                #(#registries,)*
            }
        };

        model_enum
    }

    /// Definitions for "Kind" enums for assets and models
    fn kind(&self) -> TokenStream {
        fn kind_for(
            kind_name: &Ident,
            visibility: &Visibility,
            schema: bool,
            items: &[&Ident],
        ) -> TokenStream {
            let schema_derive = schema.then(|| quote!(#[derive(schemars::JsonSchema)]));
            let display_impl = if items.is_empty() {
                quote!(unreachable!())
            } else {
                quote! {
                        write!(f, "{}", match self {
                            #(#kind_name::#items => stringify!(#items),)*
                        })
                }
            };
            let kind_enum = quote! {
                #[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
                #schema_derive
                #[serde(rename_all = "PascalCase")]
                #visibility enum #kind_name {
                    #(#items,)*
                }

                #[automatically_derived]
                impl std::fmt::Display for #kind_name {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        #display_impl
                    }
                }
            };

            kind_enum
        }

        let model_items = self
            .singletons
            .iter()
            .chain(self.collections.iter())
            .map(|s| &s.variant_name)
            .collect_vec();

        let model_kind = kind_for(&self.kind_name, &self.visibility, self.schema, &model_items);

        let asset_items = self.assets.iter().map(|i| &i.variant_name).collect_vec();
        let asset_kind = kind_for(
            &self.assets_kind_name,
            &self.visibility,
            self.schema,
            &asset_items,
        );

        quote! {
            #model_kind
            #asset_kind
        }
    }

    /// Kind provider implementations for registries
    fn kind_providers(&self) -> TokenStream {
        let Self {
            registry_name,
            partial_registry_name,
            singletons,
            collections,
            assets,
            kind_name,
            assets_kind_name,
            ..
        } = self;
        let reg = MOD_REGISTRY.deref();
        let model_impls = [registry_name, partial_registry_name]
            .into_iter()
            .flat_map(|impl_for| {
                singletons
                    .iter()
                    .chain(collections.iter())
                    .map(
                        move |ModelKind {
                                  span,
                                  variant_name,
                                  ty,
                                  ..
                              }| {
                            quote_spanned! {*span=>
                                #[automatically_derived]
                                impl #reg::kind::ItemKindProvider::<#ty> for #impl_for {
                                    fn kind() -> Self::ItemKind {
                                        #kind_name::#variant_name
                                    }
                                }
                            }
                        },
                    )
                    .chain(assets.iter().map(
                        move |AssetKind {
                                  span,
                                  variant_name,
                                  ty,
                                  ..
                              }| {
                            quote_spanned! {*span=>
                                #[automatically_derived]
                                impl #reg::kind::AssetKindProvider::<#ty> for #impl_for {
                                    fn asset_kind() -> Self::AssetKind {
                                        #assets_kind_name::#variant_name
                                    }
                                }
                            }
                        },
                    ))
            });

        quote! {
            #(#model_impls)*
        }
    }

    /// Definitions for the registry struct
    fn registry(&self) -> TokenStream {
        let Self {
            registry_name,
            singletons,
            collections,
            assets,
            visibility,
            ..
        } = self;
        let reg = MOD_REGISTRY.deref();
        let singletons = singletons.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::Singleton<#ty>,
                }
            },
        );
        let collections = collections.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::ItemCollection<#ty>,
                }
            },
        );
        let assets = assets.iter().map(
            |AssetKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::AssetsCollection<#ty>
                }
            },
        );
        quote! {
            #[derive(Debug)]
            #visibility struct #registry_name {
                #(#singletons)*
                #(#collections)*
                #(#assets)*
            }
        }
    }

    /// Definitions for the "partial" registry struct
    fn partial_registry(&self) -> TokenStream {
        let Self {
            partial_registry_name,
            singletons,
            collections,
            assets,
            visibility,
            ..
        } = self;
        let reg = MOD_REGISTRY.deref();
        let singletons = singletons.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::PartialSingleton<#ty>,
                }
            },
        );
        let collections = collections.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::PartialItemCollection<#ty>,
                }
            },
        );
        let assets = assets.iter().map(
            |AssetKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #field_name: #reg::AssetsCollection<#ty>
                }
            },
        );
        quote! {
            #[derive(Debug, Default)]
            #visibility struct #partial_registry_name {
                poisoned__: bool,
                #(#singletons)*
                #(#collections)*
                #(#assets)*
            }
        }
    }

    /// Methods implementations for the registry structs
    fn registry_impls(&self) -> TokenStream {
        let Self {
            registry_name,
            partial_registry_name,
            singletons,
            collections,
            assets,
            kind_name,
            assets_kind_name,
            error,
            ..
        } = self;

        let reg = MOD_REGISTRY.deref();
        let singletons = singletons.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #[automatically_derived]
                    impl #reg::SingletonHolder::<#ty> for #registry_name {
                        fn get_singleton(&self) -> &#reg::Singleton<#ty> {
                            &self.#field_name
                        }
                        fn get_singleton_mut(&mut self) -> &mut #reg::Singleton<#ty> {
                            &mut self.#field_name
                        }
                    }

                    #[automatically_derived]
                    impl #reg::PartialSingletonHolder::<#ty> for #partial_registry_name {
                        fn get_singleton(&mut self) -> &mut #reg::PartialSingleton<#ty> {
                            &mut self.#field_name
                        }
                    }
                }
            },
        );

        let collections = collections.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #[automatically_derived]
                    impl #reg::CollectionHolder::<#ty> for #registry_name {
                        fn get_collection(&self) -> &#reg::ItemCollection<#ty> {
                            &self.#field_name
                        }
                        fn get_collection_mut(&mut self) -> &mut #reg::ItemCollection<#ty> {
                            &mut self.#field_name
                        }
                    }

                    #[automatically_derived]
                    impl #reg::PartialCollectionHolder::<#ty> for #partial_registry_name {
                        fn get_collection(&mut self) -> &mut #reg::PartialItemCollection<#ty> {
                            &mut self.#field_name
                        }
                    }
                }
            },
        );

        let assets = assets.iter().map(
            |AssetKind {
                 span,
                 field_name,
                 ty,
                 ..
             }| {
                quote_spanned! {*span=>
                    #[automatically_derived]
                    impl #reg::AssetsHolder::<#ty> for #registry_name {
                        fn get_assets(&self) -> &#reg::AssetsCollection<#ty> {
                            &self.#field_name
                        }

                        fn get_assets_mut(&mut self) -> &mut #reg::AssetsCollection<#ty> {
                            &mut self.#field_name
                        }
                    }
                }
            },
        );

        quote! {
            #(#singletons)*
            #(#collections)*
            #(#assets)*

            #[automatically_derived]
            impl #reg::SerializationRegistry for #registry_name {
                type ItemKind = #kind_name;
                type AssetKind = #assets_kind_name;
                type Error = #error;
            }

            #[automatically_derived]
            impl #reg::SerializationRegistry for #partial_registry_name {
                type ItemKind = #kind_name;
                type AssetKind = #assets_kind_name;
                type Error = #error;
            }

            impl #reg::PartialRegistry for #partial_registry_name {
                fn poison(&mut self) {
                    self.poisoned__ = true;
                }

                fn is_poisoned(&self) -> bool {
                    self.poisoned__
                }
            }
        }
    }

    /// Implementation for "finalize" method on partial registry
    fn partial_finalize(&self) -> TokenStream {
        let Self {
            registry_name,
            partial_registry_name,
            singletons,
            collections,
            assets,
            ..
        } = self;

        let reg = MOD_REGISTRY.deref();
        let err = MOD_ERRORS.deref();

        let (col_process, col_convert): (Vec<TokenStream>,Vec<TokenStream>) = collections.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                ..
             }| {
                let process = quote_spanned! {*span=>
                    #reg::finalize::process_raw_collection::<#ty, #partial_registry_name>(&mut registry)?;
                };
                let convert = quote_spanned!{*span=>
                        let #field_name = #reg::finalize::convert_partial_collection::<#ty, #partial_registry_name>(registry.#field_name)?;
                };
                (process, convert)
            },
        ).unzip();
        let (single_process, single_convert): (Vec<TokenStream>,Vec<TokenStream>) = singletons.iter().map(
            |ModelKind {
                 span,
                 field_name,
                 ty,
                ..
             }| {
                let process = quote_spanned! {*span=>
                    #reg::finalize::process_raw_singleton::<#ty, #partial_registry_name>(&mut registry)?;
                };
                let convert = quote_spanned!{*span=>
                        let #field_name = #reg::finalize::convert_partial_singleton::<#ty, #partial_registry_name>(registry.#field_name)?;
                };
                (process, convert)
            },
        ).unzip();
        let assets_convert = assets.iter().map(
            |AssetKind {
                 span, field_name, ..
             }| {
                quote_spanned! {*span=>
                    let #field_name = registry.#field_name;
                }
            },
        );
        let field_names = collections
            .iter()
            .chain(singletons.iter())
            .map(|m| &m.field_name)
            .chain(assets.iter().map(|a| &a.field_name));

        quote! {
            impl #partial_registry_name {
                pub fn into_registry(self) -> Result<#registry_name, #err::DeserializationError<#partial_registry_name>> {
                    let mut registry = self;

                    #(#col_process)*
                    #(#single_process)*

                    #(#col_convert)*
                    #(#single_convert)*
                    #(#assets_convert)*

                    Ok(#registry_name {
                        #(#field_names,)*
                    })
                }
            }
        }
    }

    /// Implementation for model insertion method on partial registry
    fn insert_impl(&self) -> TokenStream {
        let Self {
            partial_registry_name,
            serialized_model_name,
            collections,
            singletons,
            ..
        } = self;

        let reg = MOD_REGISTRY.deref();
        let err = MOD_ERRORS.deref();

        let cols = collections.iter().map(|ModelKind{ span, variant_name, ty, .. }| {
            quote_spanned! {*span=>
                #serialized_model_name::#variant_name(item) => #reg::insert::registry_insert::<#ty, #partial_registry_name>(registry, path, item)?
            }
        });

        let singles = singletons.iter().map(|ModelKind{ span, variant_name, ty, .. }| {
            quote_spanned! {*span=>
                #serialized_model_name::#variant_name(item) => #reg::insert::singleton_insert::<#ty, #partial_registry_name>(registry, path, item)?
            }
        });

        quote! {
            impl #partial_registry_name {
                pub fn insert(&mut self, path: impl Into<#reg::path_identifier::PathIdentifier>, item: #serialized_model_name) -> Result<(), #err::DeserializationError<#partial_registry_name>> {
                    let registry = self;
                    let path = path.into();

                    match item {
                        #(#cols,)*
                        #(#singles,)*
                    }

                    Ok(())
                }
            }
        }
    }

    /// Type aliases for item IDs
    fn item_ids(&self) -> TokenStream {
        let Self {
            collections,
            visibility,
            ..
        } = self;

        let reg = MOD_REGISTRY.deref();
        let entries = collections.iter().map(
            |ModelKind {
                 span,
                 variant_name,
                 ty,
                 id_name,
                ..
             }| {
                quote_spanned! {*span=>
                    #visibility type #id_name = #reg::CollectionItemId<#ty>;
                }
            },
        );

        quote! {
            #(#entries)*
        }
    }
}

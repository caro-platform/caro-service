mod peer;
mod service;

use proc_macro::TokenStream;
use quote::quote;
use syn::{self, DeriveInput};

// --------------- SERVICE -------------------
#[proc_macro_derive(Service, attributes(service, signal, state, peer))]
pub fn service(input: TokenStream) -> TokenStream {
    let service_struct: DeriveInput = syn::parse(input).unwrap();

    let struct_fields = match service_struct.data {
        syn::Data::Struct(structure) => match structure.fields {
            syn::Fields::Named(named) => named,
            _ => unreachable!("Can't have unnamed fields in a struct"),
        },
        syn::Data::Enum(..) => panic!("Invalid 'Service' usage. Expected struct, got enum"),
        syn::Data::Union(..) => panic!("Invalid 'Service' usage. Expected struct, got union"),
    };

    if service_struct.attrs.len() != 1 {
        panic!("Invalid service name attribute. Should be a single `service_name` attribute with a service name. E.g. #[service_name(\"com.test.service\")]");
    }

    let struct_ident = &service_struct.ident;
    let (service_name, features) = service::parse_name_and_features(&service_struct.attrs[0]);

    let signals = service::parse_signals(&struct_fields);
    let states = service::parse_states(&struct_fields);
    let peers = service::parse_peers(&struct_fields);

    if features.methods {
        quote! {
            #[async_trait]
            impl karo_service::Service for Pin<Box<#struct_ident>>
                where Self: karo_service::service::ServiceMethods {
                fn service_name() -> &'static str {
                    #service_name
                }

                async fn register_service(&mut self) -> karo_bus_lib::Result<()> {
                    Self::register_bus().await?;
                    self.register_methods(Self::service_name()).await?;

                    #(#peers);*
                    #(#signals);*
                    #(#states);*
                    Ok(())
                }
            }
        }
    } else {
        quote! {
            #[async_trait]
            impl karo_service::Service for #struct_ident {
                fn service_name() -> &'static str {
                    #service_name
                }

                async fn register_service(&mut self) -> karo_bus_lib::Result<()> {
                    Self::register_bus().await?;

                    #(#peers);*
                    #(#signals);*
                    #(#states);*
                    Ok(())
                }
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn method(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn service_impl(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let imp: syn::ItemImpl = syn::parse(input.clone()).unwrap();

    let self_name = service::parse_impl_ident(&imp.self_ty);
    let methods = service::parse_methods(&imp.items);

    quote! {
        #imp

        #[async_trait]
        impl karo_service::service::ServiceMethods for Pin<Box<#self_name>> {
            async fn register_methods(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
                let context = karo_service::this::This { pointer: self };

                #(#methods);*
                Ok(())
            }
        }
    }
    .into()
}

// --------------- PEER -------------------
#[proc_macro_derive(Peer, attributes(peer, method))]
pub fn peer(input: TokenStream) -> TokenStream {
    let service_struct: DeriveInput = syn::parse(input).unwrap();

    let struct_fields = match service_struct.data {
        syn::Data::Struct(structure) => match structure.fields {
            syn::Fields::Named(named) => named,
            _ => unreachable!("Can't have unnamed fields in a struct"),
        },
        syn::Data::Enum(..) => panic!("Invalid 'Peer' usage. Expected struct, got enum"),
        syn::Data::Union(..) => panic!("Invalid 'Peer' usage. Expected struct, got union"),
    };

    if service_struct.attrs.len() != 1 {
        panic!("Invalid peer name attribute. Should be a single 'peer' attribute with a peer name. E.g. #[peer(\"com.test.peer\")]");
    }

    let struct_ident = &service_struct.ident;
    let (peer_name, features) = peer::parse_name_and_features(&service_struct.attrs[0]);

    let procedures = peer::parse_remote_methods(&struct_fields);

    let peer_impl = if features.subscriptions {
        quote! {
            #[async_trait]
            impl karo_service::peer::Peer for Pin<Box<#struct_ident>>
                where Pin<Box<#struct_ident>>: karo_service::peer::PeerSignalsAndStates {
                async fn register(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
                    let peer = Self::register_peer(service_name, #peer_name).await?;
                    self.register_callbacks(service_name).await?;

                    #(#procedures);*
                    Ok(())
                }
            }
        }
    } else {
        quote! {
            #[async_trait]
            impl karo_service::peer::Peer for Pin<Box<#struct_ident>> {
                async fn register(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
                    let peer = Self::register_peer(service_name, #peer_name).await?;

                    #(#procedures);*
                    Ok(())
                }
            }
        }
    };

    quote! {
        impl karo_service::peer::PeerName for Pin<Box<#struct_ident>> {
            fn peer_name() -> &'static str { #peer_name }
        }

        #peer_impl
    }
    .into()
}

#[proc_macro_attribute]
pub fn signal(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn state(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn peer_impl(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let imp: syn::ItemImpl = syn::parse(input.clone()).unwrap();

    let self_name = peer::parse_impl_ident(&imp.self_ty);
    let signals = peer::parse_signal_subscriptions(&imp.items);
    let states = peer::parse_state_watches(&imp.items);

    quote! {
        #imp

        #[async_trait]
        impl karo_service::peer::PeerSignalsAndStates for Pin<Box<#self_name>>
            where Pin<Box<#self_name>>: karo_service::peer::PeerName {
            async fn register_callbacks(&mut self, service_name: &str) -> karo_bus_lib::Result<()> {
                let context = karo_service::this::This { pointer: self };

                #(#signals);*
                #(#states);*

                Ok(())
            }
        }
    }
    .into()
}

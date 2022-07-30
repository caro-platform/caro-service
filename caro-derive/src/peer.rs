use quote::{quote, ToTokens};
use syn::{self, spanned::Spanned};

#[derive(Default)]
pub(crate) struct Features {
    pub subscriptions: bool,
}

pub(crate) fn parse_remote_methods(fields: &syn::FieldsNamed) -> Vec<proc_macro2::TokenStream> {
    let mut result = vec![];

    for field in fields.named.iter() {
        for attribute in field.attrs.iter() {
            if attribute.path.is_ident("method") {
                let procedure_field = field.ident.as_ref().unwrap();
                let procedure_name =
                    syn::LitStr::new(&procedure_field.to_string(), procedure_field.span());

                result.push(quote! {
                    self.#procedure_field.init(#procedure_name, peer)?;
                })
            }
        }
    }

    result
}

fn parse_features(exp: &syn::Expr) -> Features {
    let mut result = Features::default();

    if let syn::Expr::Array(syn::ExprArray {
        attrs: _,
        bracket_token: _,
        elems,
    }) = exp
    {
        for elem in elems {
            if let syn::Expr::Lit(syn::ExprLit {
                attrs: _,
                lit: syn::Lit::Str(feature),
            }) = elem
            {
                if &feature.value() == "subscriptions" {
                    result.subscriptions = true
                } else {
                    panic!("Unknown service attribute: {}", feature.value())
                }
            } else {
                panic!("Invalid features attribute. Should be a strings. E.g. #[peer(name = \"com.test.peer\", features = [\"subscriptions\"])]. Got: {}", elem.to_token_stream().to_string());
            }
        }
    } else {
        panic!("Invalid features attribute. Should be and array of strings. E.g. #[peer(name = \"com.test.peer\", features = [\"subscriptions\"])]. Got: {}", exp.to_token_stream().to_string());
    }

    result
}

fn parse_named_attributes(
    elements: syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
) -> (syn::LitStr, Features) {
    let mut features = Features::default();
    let mut name = syn::LitStr::new("", elements.span());

    for expr in elements {
        if let syn::Expr::Assign(syn::ExprAssign {
            attrs: _,
            eq_token: _,
            left,
            right,
        }) = expr
        {
            if let syn::Expr::Path(syn::ExprPath {
                attrs: _,
                qself: _,
                path,
            }) = *left
            {
                if path.is_ident("name") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        attrs: _,
                        lit: syn::Lit::Str(lit_name),
                    }) = *right
                    {
                        name = lit_name
                    } else {
                        panic!(
                            "Invalid peer name. Should be a string. E.g. #[peer(name = \"com.test.peer\")]. Got: {}",
                            right.to_token_stream().to_string()
                        )
                    }
                } else if path.is_ident("features") {
                    features = parse_features(&*right);
                } else {
                    panic!(
                        "Unknown service attribute: {}",
                        path.to_token_stream().to_string()
                    );
                }
            } else {
                panic!("Invalid named peer attribute. Should be attributes with a service name and optional features. E.g. #[peer(name = \"com.test.peer\", features = [])]");
            }
        } else {
            panic!("Invalid named peer attribute. Should be attributes with a service name and optional features. E.g. #[peer(name = \"com.test.peer\", features = [])]");
        }
    }

    if &name.value() == "" {
        panic!("Empty peer name. Should be set by attributes with a service name and optional features. E.g. #[peer(name = \"com.test.peer\", features = [])]")
    }

    (name, features)
}

pub(crate) fn parse_name_and_features(attribute: &syn::Attribute) -> (syn::LitStr, Features) {
    match attribute.path.get_ident() {
        Some(ident) => {
            if ident != "peer" {
                panic!("Invalid peer attribute. Should be a single `peer` attribute with a service name and optional features. #[peer(\"com.test.peer\")] or #[peer(name = \"com.test.peer\", features = [])]. Got: '{}'", ident);
            }

            match syn::parse2::<syn::Expr>(attribute.tokens.clone()) {
                Ok(syn::Expr::Paren(exp)) => match  *exp.expr {
                    syn::Expr::Lit(syn::ExprLit{attrs: _, lit: syn::Lit::Str(name)}) => {
                        return (name, Features::default())
                    },
                    syn::Expr::Assign(e) => {
                        // Just create a tuple struct here and parse with external function
                        let mut name_attrs = syn::punctuated::Punctuated::new();
                        name_attrs.push(syn::Expr::Assign(e));
                        return parse_named_attributes(name_attrs);
                    }
                    e => panic!("Invalid peer attribute. Should be a single 'peer' attribute with a service name and optional features. #[peer(\"com.test.peer\")] or #[peer(name = \"com.test.peer\", features = [])]. Got: '{:?}'", e)
                },
                Ok(syn::Expr::Tuple(syn::ExprTuple { attrs: _, paren_token: _, elems })) => {
                    return parse_named_attributes(elems);
                }
                Ok(e) => {
                    panic!("Invalid peer attribute. Should be a single 'peer' attribute with a service name and optional features. #[peer(\"com.test.peer\")] or #[peer(name = \"com.test.peer\", features = [])]. Got: '{:?}'", e)
                }
                Err(err) => {
                    panic!("Invalid peer attribute. Should be a single 'peer' attribute with a service name and optional features. #[peer(\"com.test.peer\")] or #[peer(name = \"com.test.peer\", features = [])]. Got: '{}'. Err: {}", attribute.tokens, err.to_string())
                }
            };
        }
        _ => {
            panic!("Invalid peer attribute. Should be a single 'peer' attribute with a service name and optional features. #[peer(\"com.test.peer\")] or #[peer(name = \"com.test.peer\", features = [])]");
        }
    }
}

pub(crate) fn parse_impl_ident(impl_type: &syn::Type) -> syn::Ident {
    let path = match impl_type {
        syn::Type::Path(syn::TypePath { qself: _, path }) => path,
        _ => panic!("Failed to parse peer subscriptions. Unexpected impl struct"),
    };

    path.get_ident().cloned().unwrap()
}

fn check_method_params(params: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>) {
    if params.len() != 2 {
        panic!("Invalid peer subscriptions signature. Should be a single-argument method. E.g 'async fn hello(&mut self, param: String) -> i32. Got: {}", params.to_token_stream().to_string());
    }

    if !matches!(params.first().unwrap(), syn::FnArg::Receiver(_)) {
        panic!("Invalid peer subscriptions method argument. Should be '&self'");
    }
}

pub(crate) fn parse_signal_subscriptions(
    methods: &Vec<syn::ImplItem>,
) -> Vec<proc_macro2::TokenStream> {
    let mut result = vec![];

    for item in methods {
        if let syn::ImplItem::Method(method) = item {
            for attr in &method.attrs {
                if attr.path.is_ident("signal") {
                    check_method_params(&method.sig.inputs);

                    let method_ident = &method.sig.ident;
                    let method_name =
                        syn::LitStr::new(&method_ident.to_string(), method_ident.span());

                    result.push(quote! {
                        Self::subscribe_on_signal(service_name, Self::peer_name(), #method_name, move |p| async move {
                            context.get().#method_ident(p).await
                        })
                        .await?;
                    })
                }
            }
        }
    }

    result
}

pub(crate) fn parse_state_watches(methods: &Vec<syn::ImplItem>) -> Vec<proc_macro2::TokenStream> {
    let mut result = vec![];

    for item in methods {
        if let syn::ImplItem::Method(method) = item {
            for attr in &method.attrs {
                if attr.path.is_ident("state") {
                    check_method_params(&method.sig.inputs);

                    let method_ident = &method.sig.ident;
                    let method_name =
                        syn::LitStr::new(&method_ident.to_string(), method_ident.span());

                    result.push(quote! {
                        let current_state = Self::watch_state(service_name, Self::peer_name(), #method_name, move |p| async move {
                            context.get().#method_ident(p).await
                        })
                        .await?;

                        self.#method_ident(current_state).await;
                    })
                }
            }
        }
    }

    result
}

use proc_macro2::{TokenStream, Span, Ident};
use syn::{parse_macro_input, Generics, DeriveInput, Attribute};
use quote::quote;
use proc_macro_helpers::{BareList};

fn get_attribute<'a>(attrs: &'a [Attribute], to_find: &str) -> Option<&'a Attribute> {
    for attr in attrs {
        let path = &attr.path();
        if let Some(ident) = path.get_ident() {
            if ident == to_find {
                return Some(attr);
            }
        }
    }
    return None;
}

fn get_handled_messages(attrs: &[Attribute]) -> Option<syn::Result<BareList<syn::Type>>> {
    get_attribute(attrs, "pt_handles").map(|attr| {
        let handled_messages: BareList::<syn::Type> = attr.parse_args()?;
        Ok(handled_messages)
    })
}

// pt_init specifies a list of requests that must be supported during init of this handler. Sending
// other types of messages (e.g. events) is not supported during handler init.
fn get_init_requests(attrs: &[Attribute]) -> Option<syn::Result<BareList<syn::Type>>> {
    get_attribute(attrs, "pt_init").map(|attr| {
        let init: BareList::<syn::Type> = attr.parse_args()?;
        Ok(init)
    })
}

fn get_init_config(attrs: &[Attribute]) -> Option<syn::Result<syn::Type>> {
    get_attribute(attrs, "pt_config").map(|attr| {
        let init_config: syn::Type = attr.parse_args()?;
        Ok(init_config)
    })
}

fn assert_not_generic(ast: &DeriveInput) {
    if ast.generics != Generics::default() {
        panic!("Generic handlers are not supported");
    }
}

fn invert_option_result<T, E>(a: Option<Result<T, E>>) -> Result<Option<T>, E> {
    if let Some(r) = a {
        match r {
            Ok(o) => Ok(Some(o)),
            Err(e) => Err(e)
        }
    } else {
        Ok(None)
    }
}

fn try_handler_macro(ast: DeriveInput) -> syn::Result<TokenStream> {
    let ident = ast.ident;

    let handled_messages: Vec<_> = invert_option_result(get_handled_messages(&ast.attrs))?
        .into_iter()
        .map(|p| p.values)
        .flatten()
        .collect();

    let init_requests: Vec<_> = invert_option_result(get_init_requests(&ast.attrs))?
        .into_iter()
        .map(|p| p.values)
        .flatten()
        .collect();

    let init_config = invert_option_result(get_init_config(&ast.attrs))?;

    let has_init_config = init_config.is_some();

    let init_config_type_snippet = if let Some(init_config) = init_config {
        quote!(type InitConfig = #init_config;)
    } else {
        quote!(type InitConfig = ();)
    };

    let impl_init_ctx_struct_snippet = if init_requests.len() > 0 {
        quote!(
            pub struct InitCtx<'a, Ctx> where Ctx: C, Ctx: 'a {
                pub ctx: &'a Ctx,
            }
    
            // implement Handle for each init request
            #(
                impl<'a, Ctx> ::context_structs::CtxHandle<#init_requests> for InitCtx<'a, Ctx> where Ctx: C, Ctx: 'a {
                    fn handle(&self, message: #init_requests) -> <#init_requests as ::message_structs::Message>::Response {
                        self.ctx.handle(message)
                    }
                }
            )*
        )
    } else {
        quote!()
    };

    let hidden_mod = Ident::new(&format!("_pt_{}", ident), Span::call_site());

    let init_ctx_struct_snippet = if init_requests.len() > 0 {
        quote!(
            type InitCtx<'a, Ctx> = #hidden_mod::InitCtx<'a, Ctx> where Self: 'a, Ctx: C, Ctx: 'a;
        )
    } else {
        quote!(
            type InitCtx<'a, Ctx> = () where Self: 'a, Ctx: C, Ctx: 'a;
        )
    };


    Ok(quote!(
        mod #hidden_mod {
            use super::*;
            #impl_init_ctx_struct_snippet
        }
        impl ::handler_structs::Handler for #ident {
            #init_config_type_snippet
            #init_ctx_struct_snippet

            fn get_handler_spec(messages_in_context: &[&'static ::message_structs::MessageSpec]) -> ::handler_structs::HandlerSpec {
                let handled_messages: &[& 'static ::message_structs::MessageSpec] = &[#(<#handled_messages as ::message_structs::Message>::get_message_spec()),*];
                let init_requests: &[& 'static ::message_structs::MessageSpec] = &[#(<#init_requests as ::message_structs::Message>::get_message_spec()),*];

                let handled_messages_in_context = handled_messages.into_iter()
                    .filter(|spec| messages_in_context.iter().any(|o| o.name == spec.name))
                    .map(|spec| *spec);

                let init_requests_in_context = init_requests.into_iter()
                    .filter(|spec| messages_in_context.iter().any(|o| o.name == spec.name))
                    .map(|spec| *spec);

                ::handler_structs::HandlerSpec {
                    name: concat!("::", module_path!(), "::", stringify!(#ident)),
                    handled_messages: handled_messages_in_context.collect(),
                    init_requests: init_requests_in_context.collect(),
                    has_init_config: #has_init_config,
                    span: proc_macro2::Span::call_site(),
                }
            }
        }

        #(

        impl ::handler_structs::hidden::DeclaredHandle<#handled_messages> for #ident {}

        )*
    ))
}

#[proc_macro_derive(Handler, attributes(pt_handles, pt_init, pt_config))]
pub fn handler_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    assert_not_generic(&ast);

    match try_handler_macro(ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}


// The format of the Handler proc_macro is
// #[derive(Handler)]
// #[pt_handles(Add1, Times3)]
// pub struct ArithmeticHandler {}

// Here pt_handles tell the event system which messages this handler can handle. Add1 and Times3 are defined in example-messages/src/lib.rs
// The information passed to the Handler proc_macro is used along with all the valid messages in the context to generate the HandlerSpec struct.
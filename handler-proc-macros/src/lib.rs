use proc_macro2::TokenStream;
use syn::{parse_macro_input, Generics, DeriveInput, Attribute};
use quote::quote;
use proc_macro_helpers::ParenList;

fn get_attribute<'a>(attrs: &'a [Attribute], to_find: &str) -> Option<&'a Attribute> {
    for attr in attrs {
        let path = &attr.path;
        if let Some(ident) = path.get_ident() {
            if ident == to_find {
                return Some(attr);
            }
        }
    }
    return None;
}

fn get_handled_messages(attrs: &[Attribute]) -> Option<syn::Result<ParenList<syn::Type>>> {
    get_attribute(attrs, "pt_handles").map(|attr| {
        let handled_messages: ParenList::<syn::Type> = syn::parse2(attr.tokens.clone())?;
        Ok(handled_messages)
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


    Ok(quote!(
        impl ::handler_structs::Handler for #ident {
            fn get_handler_spec(messages_in_context: &[&'static ::message_structs::MessageSpec]) -> ::handler_structs::HandlerSpec {
                let handled_messages: &[& 'static ::message_structs::MessageSpec] = &[#(<#handled_messages as ::message_structs::Message>::get_message_spec()),*];

                let handled_messages_in_context = handled_messages.into_iter()
                    .filter(|spec| messages_in_context.iter().any(|o| o.name == spec.name))
                    .map(|spec| *spec);

                ::handler_structs::HandlerSpec {
                    name: concat!("::", module_path!(), "::", stringify!(#ident)),
                    handled_messages: handled_messages_in_context.collect(),
                }
            }
        }

        #(

        impl ::handler_structs::hidden::DeclaredHandle<#handled_messages> for #ident {}

        )*
    ))
}

#[proc_macro_derive(Handler, attributes(pt_handles))]
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
use message_structs::MessageSpec;
use proc_macro::{self, Group};
use proc_macro2::TokenStream;
use syn::{parse_macro_input, DeriveInput, Attribute, Generics, DataStruct, token::Token, TypeTuple};
use quote::{quote, ToTokens};

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

fn has_attribute(attrs: &[Attribute], to_find: &str) -> bool {
    get_attribute(attrs, to_find).is_some()
}

fn is_async(attrs: &[Attribute]) -> bool {
    has_attribute(attrs, "pt_async")
}

fn assert_not_generic(ast: &DeriveInput) {
    if ast.generics != Generics::default() {
        panic!("Generic messages are not supported");
    }
}

fn parse_pt_response(attr: &Attribute) -> syn::Result<syn::Type> {
    let response_type: syn::TypeParen = syn::parse2(attr.tokens.clone())?;
    Ok(*response_type.elem)
}

fn get_response_type(attrs: &[Attribute]) -> Option<syn::Result<syn::Type>> {
    get_attribute(attrs, "pt_response").map(parse_pt_response)
}

fn try_message_macro(ast: DeriveInput) -> syn::Result<TokenStream> {
    
    let is_async = is_async(&ast.attrs);
    let ident = ast.ident;
    let response_type = match get_response_type(&ast.attrs) {
        Some(Ok(t)) => quote! {#t},
        Some(Err(e)) => return Err(e),
        None => quote!{()},
    };

    Ok(quote!(
        impl ::message_structs::Message for #ident {
            type Response = #response_type;

            fn name() -> &'static str {
                concat!(module_path!(), "::", stringify!(#ident))
            }

            fn is_async() -> bool {
                #is_async
            }
        }
    ))
}

#[proc_macro_derive(Message, attributes(pt_async, pt_response))]
pub fn message_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    
    assert_not_generic(&ast);
    dbg!(&ast);

    match try_message_macro(ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}


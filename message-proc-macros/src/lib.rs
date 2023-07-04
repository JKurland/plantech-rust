use proc_macro2::TokenStream;
use syn::{parse_macro_input, DeriveInput, Attribute, Generics};
use quote::quote;

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
    !has_attribute(attrs, "pt_sync")
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

    let (response_type, has_response) = match get_response_type(&ast.attrs) {
        Some(Ok(t)) => (quote! {#t}, true),
        Some(Err(e)) => return Err(e),
        None => (quote!{()}, false),
    };

    let wrapped_response_type = if is_async {
        quote!{::futures::future::BoxFuture<'static, #response_type>}
    } else {
        response_type.clone()
    };

    Ok(quote!(
        impl ::message_structs::Message for #ident {
            type Response = #wrapped_response_type;
            type UnwrappedResponse = #response_type;

            fn get_message_spec() -> &'static ::message_structs::MessageSpec {
                static s: ::message_structs::MessageSpec = ::message_structs::MessageSpec {
                    name: concat!("::", module_path!(), "::", stringify!(#ident)),
                    is_async: #is_async,
                    has_response: #has_response,
                };
                &s
            }
        }
    ))
}


#[proc_macro_derive(Message, attributes(pt_sync, pt_response))]
pub fn message_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    
    assert_not_generic(&ast);

    match try_message_macro(ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

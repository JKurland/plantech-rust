use handler_structs::HandlerSpec;
use message_structs::MessageSpec;
use proc_macro2::{TokenStream, Ident, Span};
use quote::quote;
use syn::{TypePath, parse_str, Expr};

pub fn context_impl(message_specs: Vec<&'static MessageSpec>, handler_specs: Vec<HandlerSpec>) -> syn::Result<TokenStream> {
    let handler_names_iter = || (0..handler_specs.len())
        .map(|i| Ident::new(&format!("handler{}", i), Span::call_site()));

    let handler_exprs = handler_names_iter()
        .map(|ident| parse_str::<Expr>(&format!("self.{}", ident)))
        .collect::<syn::Result<Vec<_>>>()?;

    let handler_type_names = handler_specs.iter().map(|spec| parse_str(spec.name)).collect::<syn::Result<Vec<TypePath>>>()?;

    let handle_impls = message_specs.iter()
        .map(
            |message_spec| {
                let (handler_exprs, handler_types): (Vec<_>, Vec<_>) = handler_specs
                    .iter()
                    .zip(handler_exprs.iter().zip(handler_type_names.iter()))
                    .filter(|(handler, _)| handler.handled_messages.iter().any(|spec| &spec.name == &message_spec.name))
                    .map(|(_, i)| i)
                    .unzip();

                
                let message_name: TypePath = parse_str(message_spec.name)?;

                let handle_body = match (message_spec.has_response, handler_exprs.as_slice(), handler_types.as_slice()) {
                    (false, handler_exprs, handler_types) => {
                        quote!(#( < #handler_types as ::handler_structs::Handle::<#message_name> >::handle(&#handler_exprs, message); )*)
                    },
                    (true, [handler_expr], [handler_type]) => {
                        quote!( < #handler_type as ::handler_structs::Handle::<#message_name> >::handle(&#handler_expr, message))
                    },
                    (true, [], []) => {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!("Message ({}) with a response type has no handlers", message_spec.name)
                        ));
                    },
                    (true, _, _) => {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!("Message ({}) with a response type has multiple handlers {:?}", message_spec.name, handler_types)
                        ));
                    }
                };

                Ok(quote!(
                    impl Handle<#message_name> for Context {
                        fn handle(&self, message: #message_name) -> <#message_name as ::message_structs::Message>::Response {
                            #handle_body
                        }
                    }
                ))
            }
        )
        .reduce(|a: syn::Result<TokenStream>, b| {
            match (&a, &b) {
                (Err(_), _) => a,
                (_, Err(_)) => b,
                (Ok(ok_a), Ok(ok_b)) => Ok(quote!(#ok_a #ok_b))
            }
        })
        .unwrap_or(Ok(quote!()))?;
    
    let handler_names = handler_names_iter();
    
    Ok(quote!(
        struct Context {
            #( #handler_names: #handler_type_names ),*
        }

        trait Handle<T: ::message_structs::Message> {
            fn handle(&self, message: T) -> T::Response;
        }

        #handle_impls
    ))
}
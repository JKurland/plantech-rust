use handler_structs::HandlerSpec;
use message_structs::MessageSpec;
use proc_macro2::{TokenStream, Ident, Span};
use quote::quote;
use syn::{TypePath, parse_str, Expr};

struct Handler<'a> {
    spec: &'a HandlerSpec,
    member_name: Ident,
    get_member_expr: Expr,
    type_name: TypePath,
}

impl<'a> Handler<'a> {
    fn handles(&self, message_spec: &MessageSpec) -> bool {
        self.spec.handled_messages.iter().any(|spec| &spec.name == &message_spec.name)
    }

    fn from_handler_spec(handler_spec: &'a HandlerSpec, i: usize) -> Self {
        // member name is handler_i
        // get_member_expr is self.handler_i
        // type name comes from handler_spec.name
        let member_name = Ident::new(&format!("handler_{}", i), Span::call_site());
        let get_member_expr = parse_str(&format!("self.{}", member_name)).unwrap();
        let type_name: TypePath = parse_str(&handler_spec.name).unwrap();

        Self {
            spec: handler_spec,
            member_name,
            get_member_expr,
            type_name,
        }
    }
}


fn make_handle_impl(message_spec: &MessageSpec, handlers: &[Handler]) -> syn::Result<TokenStream> {
    // get an iter of handlers which handle this message
    let handlers = handlers.iter().filter(|h| h.handles(message_spec)).collect::<Vec<_>>();

    let message_name: TypePath = parse_str(message_spec.name)?;

    let handle_body = match (message_spec.has_response, handlers.as_slice()) {
        (false, handlers) => {
            let handler_types = handlers.iter().map(|h| &h.type_name);
            let handler_exprs = handlers.iter().map(|h| &h.get_member_expr);
            quote!(#( < #handler_types as ::handler_structs::Handle::<#message_name> >::handle(&#handler_exprs, self, message); )*)
        },
        (true, [handler]) => {
            let handler_type = &handler.type_name;
            let handler_expr = &handler.get_member_expr;
            quote!( < #handler_type as ::handler_structs::Handle::<#message_name> >::handle(&#handler_expr, self, message))
        },
        (true, []) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Message ({}) with a response type has no handlers", message_spec.name)
            ));
        },
        (true, _) => {
            let handler_types = handlers.iter().map(|h| &h.type_name).collect::<Vec<_>>();
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Message ({}) with a response type has multiple handlers {:?}", message_spec.name, handler_types)
            ));
        }
    };

    Ok(quote!(
        impl ::context_structs::Handle<#message_name> for Context {
            fn handle(&self, message: #message_name) -> <#message_name as ::message_structs::Message>::Response {
                #handle_body
            }
        }
    ))
}

pub fn context_impl(message_specs: Vec<&'static MessageSpec>, handler_specs: Vec<HandlerSpec>) -> syn::Result<TokenStream> {
    // make a vec of Handlers
    let handlers = handler_specs.iter()
        .enumerate()
        .map(|(i, spec)| Handler::from_handler_spec(spec, i))
        .collect::<Vec<_>>();

    let handle_impls = message_specs.iter()
        .map(|message_spec| {
            make_handle_impl(*message_spec, &handlers)
        })
        .reduce(|a: syn::Result<TokenStream>, b| {
            match (&a, &b) {
                (Err(_), _) => a,
                (_, Err(_)) => b,
                (Ok(ok_a), Ok(ok_b)) => Ok(quote!(#ok_a #ok_b))
            }
        })
        .unwrap_or(Ok(quote!()))?;


    let handler_names = handlers.iter().map(|h| &h.member_name).collect::<Vec<_>>();
    let handler_type_names = handlers.iter().map(|h| &h.type_name).collect::<Vec<_>>();
    Ok(quote!(
        struct Context {
            #( #handler_names: #handler_type_names ),*
        }

        impl Context {
            fn new(#( #handler_names: #handler_type_names ),*) -> Self {
                Self {
                    #( #handler_names ),*
                }
            }
        }

        impl ::message_list::C for Context {}

        #handle_impls
    ))
}

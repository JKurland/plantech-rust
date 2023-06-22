use std::collections::HashSet;

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

    fn from_handler_spec(handler_spec: &'a HandlerSpec, name: &str) -> Self {
        // member name is handler_i
        // get_member_expr is self.handler_i
        // type name comes from handler_spec.name
        let member_name = Ident::new(name, Span::call_site());
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

fn make_handle_impl_body(message_spec: &MessageSpec, handlers: &[&Handler], unwrap_member: bool) -> syn::Result<TokenStream> {
    let get_member_expr = if unwrap_member {
        |handler: &Handler<'_>| {
            let bare_expr = &handler.get_member_expr;
            quote!(#bare_expr.as_ref().unwrap())
        }
    } else {
        |handler: &Handler<'_>| {
            let bare_expr = &handler.get_member_expr;
            quote!(#bare_expr)
        }
    };

    let message_name: TypePath = parse_str(message_spec.name)?;
    Ok(match (message_spec.has_response, handlers) {
        (false, handlers) => {
            let handler_types = handlers.iter().map(|h| &h.type_name);
            let handler_exprs = handlers.iter().map(|h| get_member_expr(h));
            quote!(#( < #handler_types as ::handler_structs::Handle::<#message_name> >::handle(&#handler_exprs, self, message); )*)
        },
        (true, [handler]) => {
            let handler_type = &handler.type_name;
            let handler_expr = get_member_expr(handler);
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
    })
}

fn make_handle_impl(message_spec: &MessageSpec, handlers: &[Handler]) -> syn::Result<TokenStream> {
    // get an iter of handlers which handle this message
    let handlers = handlers.iter().filter(|h| h.handles(message_spec)).collect::<Vec<_>>();

    let message_name: TypePath = parse_str(message_spec.name)?;
    let handle_body = make_handle_impl_body(message_spec, &handlers, false)?;
    let handle_body_with_unwrap = make_handle_impl_body(message_spec, &handlers, true)?;

    Ok(quote!(
        impl ::context_structs::CtxHandle<#message_name> for Context {
            fn handle(&self, message: #message_name) -> <#message_name as ::message_structs::Message>::Response {
                #handle_body
            }
        }

        impl ::context_structs::CtxHandle<#message_name> for PartialContext {
            fn handle(&self, message: #message_name) -> <#message_name as ::message_structs::Message>::Response {
                #handle_body_with_unwrap
            }
        }
    ))
}

fn make_context_config(handlers: &[Handler]) -> TokenStream {
    let (handler_types_with_config, handler_member_names_with_config): (Vec<_>, Vec<_>) = handlers.iter()
        .filter(|handler| handler.spec.has_init_config)
        .map(|handler| {
            (&handler.type_name, &handler.member_name)
        })
        .unzip();

    quote!(
        pub struct ContextConfig {
            #(pub #handler_member_names_with_config: <#handler_types_with_config as ::handler_structs::Handler>::InitConfig),*
        }
    )
}

pub fn context_impl(message_specs: Vec<&'static MessageSpec>, handler_specs: Vec<(&'static str, HandlerSpec)>) -> syn::Result<TokenStream> {
    // make a vec of Handlers
    let handlers = handler_specs.iter()
        .map(|(name, spec)| Handler::from_handler_spec(spec, name))
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

    let context_config = make_context_config(&handlers);

    let handler_names = handlers.iter().map(|h| &h.member_name).collect::<Vec<_>>();
    let handler_type_names = handlers.iter().map(|h| &h.type_name).collect::<Vec<_>>();

    // need to check that every init request required by each handler is provided by handlers
    // which appear previously in the handler list.
    {
        let mut available_requests: HashSet<&'static str> = HashSet::new();
        for handler in handlers.iter() {
            // check that all init messages are in fact requests
            for init_request in handler.spec.init_requests.iter() {
                if !init_request.has_response {
                    return Err(syn::Error::new(
                        handler.spec.span,
                        format!("Handler {} requires init message {} which is not a request", handler.spec.name, init_request.name)
                    ));
                }
            }

            let unavailable_request = handler.spec.init_requests.iter().find(|req| !available_requests.contains(req.name));
            if let Some(r) = unavailable_request {
                return Err(syn::Error::new(
                    handler.spec.span,
                    format!("Handler {} requires request {} which is not provided by any previous handler", handler.spec.name, r.name)
                ));
            }
            available_requests.extend(
                handler.spec.handled_messages.iter()
                    .filter(|message| message.has_response) // only requests
                    .map(|request| &request.name)
            );
        }
    }

    let call_inits = handlers.iter().map(|handler| {
        let handler_name = &handler.member_name;
        let handler_type = &handler.type_name;
        let init_ctx_snippet = if handler.spec.init_requests.is_empty() {
            quote!(&())
        } else {
            quote!({
                type InitCtx<'a, Ctx: ::message_list::C + 'a> = <#handler_type as ::handler_structs::Handler>::InitCtx<'a, Ctx>;
                &InitCtx{ctx: &partial_context}
            })
        };

        let config_snippet = if handler.spec.has_init_config {
            quote!(config.#handler_name)
        } else {
            quote!(())
        };

        quote!(
            {
                let init_ctx = #init_ctx_snippet;
                partial_context.#handler_name = ::std::option::Option::Some(<#handler_type as ::handler_structs::HandlerInit>::init::<PartialContext>(init_ctx, #config_snippet));
            }
        )
    });

    Ok(quote!(
        #context_config

        #[derive(Default)]
        struct PartialContext {
            #( #handler_names: ::std::option::Option<#handler_type_names> ),*
        }

        pub struct Context {
            #( #handler_names: #handler_type_names ),*
        }

        impl Context {
            pub fn new(config: ContextConfig) -> Self {
                let mut partial_context = PartialContext::default();
                #(#call_inits)*
                Self {
                    #(#handler_names: partial_context.#handler_names.unwrap()),*
                }
            }
        }

        impl ::message_list::C for Context {}
        impl ::message_list::C for PartialContext {}

        #handle_impls
    ))
}

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

fn make_handle_impl_body_for_proxy(message_spec: &MessageSpec) -> TokenStream {
    let enum_name = any_message_enum_name(message_spec);

    let make_any_message = if message_spec.has_response {
        quote!(
            let (sender, receiver) = ::oneshot::channel();
            let any_message = AnyMessage::#enum_name(message, sender);
        )
    } else {
        quote!(
            let any_message = AnyMessage::#enum_name(message);
        )
    };

    let send_snippet = if message_spec.is_async {
        quote!(self_sender.send(any_message).await.unwrap();)
    } else {
        quote!(self.sender.try_send(any_message).unwrap();)
    };

    let return_response = if message_spec.has_response {
        if message_spec.is_async {
            quote!(receiver.await.unwrap())
        } else {
            quote!(receiver.recv().unwrap())
        }
    } else {
        quote!()
    };

    if message_spec.is_async {
        quote!(
            use ::futures::FutureExt;
            let self_sender = self.sender.clone();
            async move {
                #make_any_message
                #send_snippet
                #return_response
            }.boxed()
        )
    } else {
        quote!(
            #make_any_message
            #send_snippet
            #return_response
        )
    }
}

fn make_handle_impl(message_spec: &MessageSpec, handlers: &[Handler]) -> syn::Result<TokenStream> {
    // get an iter of handlers which handle this message
    let handlers = handlers.iter().filter(|h| h.handles(message_spec)).collect::<Vec<_>>();

    let message_name: TypePath = parse_str(message_spec.name)?;
    let handle_body = make_handle_impl_body(message_spec, &handlers, false)?;
    let handle_body_with_unwrap = make_handle_impl_body(message_spec, &handlers, true)?;
    let handle_body_proxy = make_handle_impl_body_for_proxy(message_spec);

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

        impl ::context_structs::CtxHandle<#message_name> for ContextProxy {
            fn handle(&self, message: #message_name) -> <#message_name as ::message_structs::Message>::Response {
                #handle_body_proxy
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

fn any_message_enum_name(spec: &MessageSpec) -> Ident {
    let name = &spec.name.replace("_", "__").replace("::", "_");
    let trimmed_name = name.trim_start_matches("_");
    Ident::new(trimmed_name, Span::call_site())
}

fn make_any_message_enum(message_specs: &[&'static MessageSpec]) -> TokenStream {
    let enum_types = message_specs.iter().map(|spec| {
        let message_type = syn::parse_str::<TypePath>(spec.name).unwrap();
        if spec.has_response {
            quote!(#message_type, ::oneshot::Sender<<#message_type as ::message_structs::Message>::UnwrappedResponse>)
        } else {
            quote!(#message_type)
        }
    });

    let message_idents: Vec<_> = message_specs.iter().map(|s| any_message_enum_name(s)).collect();

    let match_arms = message_specs.iter().zip(&message_idents).map(|(spec, ident)| {
        let get_response_snippet = if spec.is_async {
            quote!(ctx.handle(message).await)
        } else {
            quote!(ctx.handle(message))
        };

        if spec.has_response {
            quote!(Self::#ident(message, sender) => {
                let response = #get_response_snippet;
                // ignore the error, it just means the receiver was dropped
                let _ = sender.send(response);
            })
        } else {
            quote!(Self::#ident(message) => {
                #get_response_snippet;
            })
        }
    });

    quote!(
        pub enum AnyMessage {
            #( #message_idents ( #enum_types ) ),*
        }

        impl AnyMessage {
            pub async fn pass_to(self, ctx: &impl ::message_list::C) {
                match self {
                    #(#match_arms,)*
                }
            }
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
                type InitCtx<'a, Ctx> = <#handler_type as ::handler_structs::Handler>::InitCtx<'a, Ctx>;
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

    let any_message_enum = make_any_message_enum(&message_specs);

    Ok(quote!(
        #context_config
        #any_message_enum

        // partial context needs at least the sender so it can give out ContextProxy during init
        #[derive(Default)]
        struct PartialContext {
            #( #handler_names: ::std::option::Option<#handler_type_names> ),*,
            context_proxy_sender: ::std::option::Option<::smol::channel::Sender<AnyMessage>>,
            context_proxy_receiver: ::std::option::Option<::smol::channel::Receiver<AnyMessage>>,
        }

        pub struct Context {
            #( #handler_names: #handler_type_names ),*,
            context_proxy_sender: ::smol::channel::Sender<AnyMessage>,
            context_proxy_receiver: ::smol::channel::Receiver<AnyMessage>,
        }

        #[derive(Clone)]
        pub struct ContextProxy {
            sender: ::smol::channel::Sender<AnyMessage>,
        }

        impl Context {
            pub fn new(config: ContextConfig) -> Self {
                let (context_proxy_sender, context_proxy_receiver) = ::smol::channel::bounded(1024);
                let mut partial_context = PartialContext::default();

                partial_context.context_proxy_sender = ::std::option::Option::Some(context_proxy_sender);
                partial_context.context_proxy_receiver = ::std::option::Option::Some(context_proxy_receiver);

                #(#call_inits)*

                Self {
                    #(#handler_names: partial_context.#handler_names.unwrap()),*,
                    context_proxy_sender: partial_context.context_proxy_sender.unwrap(),
                    context_proxy_receiver: partial_context.context_proxy_receiver.unwrap(),
                }
            }

            pub async fn run(&self) {
                let executor = ::smol::LocalExecutor::new();

                loop {
                    if executor.is_empty() {
                        // if we have no tasks wait on the receiver
                        let message = match self.context_proxy_receiver.recv().await {
                            Ok(message) => message,
                            Err(_) => {return;},
                        };
                        executor.spawn(async move {
                            message.pass_to(self).await;
                        }).detach();
                    } else {
                        // otherwise check for any new messages without waiting
                        match self.context_proxy_receiver.try_recv() {
                            Ok(message) => {
                                executor.spawn(async move {
                                    message.pass_to(self).await;
                                }).detach();
                            },
                            Err(::smol::channel::TryRecvError::Empty) => {
                                if !executor.is_empty() {
                                    executor.tick().await;
                                }
                            },
                            Err(::smol::channel::TryRecvError::Closed) => {return;},
                        }
                    }
                }
            }
        }

        impl ::message_list::C for Context {
            fn proxy(&self) -> ::std::boxed::Box<dyn ::message_list::C + Send> {
                ::std::boxed::Box::new(ContextProxy {
                    sender: self.context_proxy_sender.clone(),
                })
            }

            fn quit(&self) {
                self.context_proxy_sender.close();
            }
        }

        impl ::message_list::C for PartialContext {
            fn proxy(&self) -> ::std::boxed::Box<dyn ::message_list::C + Send> {
                ::std::boxed::Box::new(ContextProxy {
                    sender: self.context_proxy_sender.clone().unwrap(),
                })
            }

            fn quit(&self) {
                self.context_proxy_sender.as_ref().unwrap().close();
            }
        }

        impl ::message_list::C for ContextProxy {
            fn proxy(&self) -> ::std::boxed::Box<dyn ::message_list::C + Send> {
                ::std::boxed::Box::new(self.clone())
            }

            fn quit(&self) {
                self.sender.close();
            }
        }

        #handle_impls
    ))
}

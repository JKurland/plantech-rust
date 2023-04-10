use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, Token};
use proc_macro_helpers::List;


mod kw {
    use syn::custom_keyword;

    custom_keyword!(Messages);
    custom_keyword!(Handlers);
}

struct Messages {
    _kw: kw::Messages,
    _sep: Token![:],
    value: syn::Expr,
}

impl Parse for Messages {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _kw: input.parse()?,
            _sep: input.parse()?,
            value: input.parse()?
        })
    }
}

struct Handlers {
    _kw: kw::Handlers,
    _sep: Token![:],
    types: List<syn::TypePath>
}

impl Parse for Handlers {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _kw: input.parse()?,
            _sep: input.parse()?,
            types: input.parse()?
        })
    }
}

struct DefineContextInput {
    messages: Option<Messages>,
    handlers: Option<Handlers>,
}

impl Parse for DefineContextInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut messages = None;
        let mut handlers = None;
        while !input.is_empty() {
            if input.peek(kw::Messages) {
                messages = Some(input.parse()?);
            } else if input.peek(kw::Handlers) {
                handlers = Some(input.parse()?);
            } else {
                return Err(input.error("Expected either 'Messages' or 'Handlers'"));
            }
        }
        Ok(Self { messages: messages, handlers: handlers })
    }
}


fn try_define_context_type(ts: TokenStream) -> syn::Result<TokenStream> {
    let input: DefineContextInput = syn::parse2(ts.clone())?;

    let messages_function = input.messages.ok_or(syn::Error::new_spanned(ts, "Expected 'Messages'"))?.value;

    let handler_paths = input.handlers
        .iter()
        .map(|handlers| &handlers.types.values)
        .flatten();

    Ok(quote!(
        #[proc_macro]
        pub fn context_type(ts: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
            let messages = #messages_function();
            let handlers = vec![#( <#handler_paths as ::handler_structs::Handler>::get_handler_spec(&messages) ),* ];

            match context_impl::context_impl(messages, handlers) {
                Ok(ts) => ts,
                Err(err) => err.to_compile_error()
            }.into()
        }
    ))
}

fn try_message_list(ts: TokenStream) -> syn::Result<TokenStream> {
    let input: List<syn::TypePath> = syn::parse2(ts)?;

    let message_paths: Vec<_> = input.values.iter().collect();

    Ok(quote!(
        pub fn messages() -> Vec<&'static ::message_structs::MessageSpec> {
            vec![#( <#message_paths as ::message_structs::Message>::get_message_spec() ),* ]
        }

        pub trait C: #( ::context_structs::Handle<#message_paths> + )* {}
    ))
}

#[proc_macro]
pub fn define_context_type(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match try_define_context_type(TokenStream::from(ts)) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error()
    }.into()
}



#[proc_macro]
pub fn message_list(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match try_message_list(TokenStream::from(ts)) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error()
    }.into()
}

// To create a context object defined in this way you need to pass in the
// instances of each handler to the context constructor. For example
// Context::new(handler1, handler2, handler3, ...)
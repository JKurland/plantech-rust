use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, Token};
use proc_macro_helpers::List;

// // defines the context! macro
// define_context!(
//     Messages: [
//         ...
//     ]

//     Handlers: [
//         ...
//     ]
//
//      .. other config maybe
// )

// expands to
// #[proc_macro]
// pub fn context(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     let messages = [Messages...];
//     let handlers = [Handlers...];

//     context_impl(messages, handlers);
// }

mod kw {
    use syn::custom_keyword;

    custom_keyword!(Messages);
    custom_keyword!(Handlers);
}

struct Messages {
    _kw: kw::Messages,
    _sep: Token![:],
    types: List<syn::TypePath>
}

impl Parse for Messages {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _kw: input.parse()?,
            _sep: input.parse()?,
            types: input.parse()?
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

// let context = context!(
//     handler_expr1,
//     handler_expr2,
// )


// expands to

// let context = {
//     define context type

//     ContextType::new(
//         handler_expr1,
//         handler_expr2,
//     )
// }

fn try_define_context_type(ts: TokenStream) -> syn::Result<TokenStream> {
    let input: DefineContextInput = syn::parse2(ts)?;

    let message_paths = input.messages
        .iter()
        .map(|messages| &messages.types.values)
        .flatten();

    let handler_paths = input.handlers
        .iter()
        .map(|handlers| &handlers.types.values)
        .flatten();
    
    let r = Ok(quote!(
        #[proc_macro]
        pub fn context_type(ts: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
            let messages = vec![#( <#message_paths as ::message_structs::Message>::get_message_spec() ),* ];

            let handlers = vec![#( <#handler_paths as ::handler_structs::Handler>::get_handler_spec(&messages) ),* ];

            match context_impl::context_impl(messages, handlers) {
                Ok(ts) => ts,
                Err(err) => err.to_compile_error()
            }.into()
        }
    ));

    r
}

#[proc_macro]
pub fn define_context_type(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match try_define_context_type(TokenStream::from(ts)) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error()
    }.into()
}

use syn::{parse::Parse, Token, punctuated::Punctuated, bracketed, parenthesized};

// See context-proc-macro for example usage

// A list surrounded by [] (e.g. [1,2,3,])
pub struct List<ValueT: Parse> {
    pub bracket_token: syn::token::Bracket,
    pub values: Punctuated<ValueT, Token![,]>,
}

impl<ValueT: Parse> Parse for List<ValueT> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: bracketed!(content in input),
            values: content.parse_terminated(ValueT::parse)?
        })
    }
}


// A list surrounded by ()
pub struct ParenList<ValueT: Parse> {
    pub paren_token: syn::token::Paren,
    pub values: Punctuated<ValueT, Token![,]>,
}

impl<ValueT: Parse> Parse for ParenList<ValueT> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            values: content.parse_terminated(ValueT::parse)?
        })
    }
}

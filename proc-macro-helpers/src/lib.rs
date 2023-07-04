use std::collections::HashSet;
use std::hash::Hash;
use quote::ToTokens;

use syn::{parse::Parse, Token, punctuated::Punctuated, bracketed, parenthesized, braced};

// See context-proc-macro for example usage


// A list not surrounded by any brackets
pub struct BareList<ValueT: Parse> {
    pub values: Punctuated<ValueT, Token![,]>,
}

impl<ValueT: Parse> Parse for BareList<ValueT> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            values: input.parse_terminated(ValueT::parse, Token![,])?
        })
    }
}


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
            values: content.parse_terminated(ValueT::parse, Token![,])?
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
            values: content.parse_terminated(ValueT::parse, Token![,])?
        })
    }
}

// A single value surrounded by ()
pub struct ParenValue<ValueT: Parse> {
    pub paren_token: syn::token::Paren,
    pub value: ValueT,
}

impl<ValueT: Parse> Parse for ParenValue<ValueT> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            value: content.parse()?
        })
    }
}

// A pair of values separated by a colon
pub struct Pair<T1: Parse, T2: Parse> {
    pub first: T1,
    pub _sep: Token![:],
    pub second: T2,
}

impl<T1: Parse, T2: Parse> Parse for Pair<T1, T2> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            first: input.parse()?,
            _sep: input.parse()?,
            second: input.parse()?
        })
    }
}

// A dictionary surrounded by {}
pub struct Dict<KeyT: Parse + Eq + Hash + ToTokens, ValueT: Parse> {
    pub brace_token: syn::token::Brace,
    pub items: Punctuated<Pair<KeyT, ValueT>, Token![,]>,
}

impl<KeyT: Parse + Eq + Hash + ToTokens, ValueT: Parse> Parse for Dict<KeyT, ValueT> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let rtn = Self {
            brace_token: braced!(content in input),
            items: content.parse_terminated(Pair::parse, Token![,])?
        };

        // check for duplicate keys in items
        let mut keys = HashSet::new();
        for item in &rtn.items {
            if !keys.insert(&item.first) {
                return Err(syn::Error::new_spanned(&item.first, "Duplicate key"));
            }
        }

        Ok(rtn)
    }
}

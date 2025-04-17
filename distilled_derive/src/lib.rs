// extern crate distilled;
extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;

#[proc_macro_derive(Distilled)]
pub fn distill_derive(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let expanded = match &input.data {
        // Named‐field structs → pull out `named` directly
        Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => derive_named(ident, named),

        // Newtypes (1‑field tuple structs)
        Data::Struct(syn::DataStruct {
            fields: syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }),
            ..
        }) if unnamed.len() == 1 => {
            let ty = &unnamed.first().unwrap().ty;
            derive_newtype(ident, ty)
        }

        _ => unimplemented!("`Distilled` only supports named structs and newtypes"),
    };

    expanded.into()
}

fn derive_named(
    struct_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let num_fields = fields.len();
    let distillers = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap().to_string();
        let ident = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        quote! { let #ident = <#ty>::distill(value.get(#name)); }
    });
    let checks = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote! { #ident.is_ok() }
    });
    let unpack = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote! { #ident: #ident.unwrap() }
    });
    let collect_errors = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let name = ident.to_string();
        quote! {
          if #ident.is_err() {
            errors.insert(#name.into(), #ident.err().unwrap());
          }
        }
    });

    quote! {
        impl ::distilled::Distilled for #struct_ident {
            fn distill<'a, T: Into<Option<&'a serde_json::Value>>>(
                value: T
            ) -> Result<Self, ::distilled::Error> {
                let value = value.into().ok_or_else(|| Error::entry("missing_field"))?;
                let mut errors = std::collections::HashMap::with_capacity(#num_fields);

                #(#distillers)*

                if #(#checks)&&* {
                    Ok(#struct_ident { #(#unpack),* })
                } else {
                    #(#collect_errors)*
                    Err(Error::Struct(errors))
                }
            }
        }
    }
}

fn derive_newtype(struct_ident: &syn::Ident, inner_ty: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        impl ::distilled::Distilled for #struct_ident {
            fn distill<'a, T: Into<Option<&'a serde_json::Value>>>(
                value: T
            ) -> Result<Self, ::distilled::Error> {
                let inner = <#inner_ty>::distill(value)?;
                Ok(#struct_ident(inner))
            }
        }
    }
}

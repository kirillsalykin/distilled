extern crate proc_macro2;

use darling::ast::{Data, Style};
use darling::{FromDeriveInput, FromField};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(FromDeriveInput)]
#[darling(supports(struct_named, struct_tuple))]
struct Input {
    ident: syn::Ident,
    data: Data<(), Field>,
}

#[derive(FromField)]
struct Field {
    #[darling(default)]
    ident: Option<syn::Ident>,
    ty: syn::Type,
}

#[proc_macro_derive(Distilled)]
pub fn distill_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(item as DeriveInput);

    let input = match Input::from_derive_input(&derive_input) {
        Ok(i) => i,
        Err(err) => return err.write_errors().into(),
    };

    let name = &input.ident;

    let (style, fields) = match input.data {
        Data::Struct(ds) => ds.split(),
        _ => unreachable!("Distilled only supports structs & struct tuples"),
    };

    let expanded = match style {
        Style::Struct => expand_named(name, &fields),
        Style::Tuple => expand_tuple(name, &fields),
        _ => unreachable!(),
    };

    expanded.into()
}

fn expand_named(struct_ident: &syn::Ident, fields: &[Field]) -> proc_macro2::TokenStream {
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

fn expand_tuple(struct_ident: &syn::Ident, fields: &[Field]) -> proc_macro2::TokenStream {
    let num_fields = fields.len();

    let distillers = fields.iter().enumerate().map(|(i, f)| {
        let idx = syn::Index::from(i);
        let ty = &f.ty;
        let var = syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
        quote! { let #var = <#ty>::distill(value.get(#idx)); }
    });
    let checks = (0..num_fields).map(|i| {
        let var = syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
        quote! { #var.is_ok() }
    });
    let unpack = (0..num_fields).map(|i| {
        let var = syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
        quote! { #var.unwrap() }
    });
    let collect_errors = (0..num_fields).map(|i| {
        let var = syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
        let name = i.to_string();
        quote! {
            if #var.is_err() {
                errors.insert(#name.into(), #var.err().unwrap());
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
                    Ok(#struct_ident( #(#unpack),* ))
                } else {
                    #(#collect_errors)*
                    Err(Error::Struct(errors))
                }
            }
        }
    }
}

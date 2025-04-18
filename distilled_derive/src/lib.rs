extern crate proc_macro2;

use darling::{
    FromDeriveInput, FromField, FromMeta,
    ast::{Data, Style},
};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(FromDeriveInput)]
#[darling(supports(struct_named, struct_tuple))]
struct Input {
    ident: syn::Ident,
    data: Data<(), Field>,
}

#[derive(FromField)]
#[darling(attributes(distilled))]
struct Field {
    #[darling(default)]
    ident: Option<syn::Ident>,

    ty: syn::Type,

    #[darling(default)]
    pub email: bool,
}

// #[derive(Debug, FromMeta)]
// #[darling(rename_all = "lowercase")]
// enum Rule {
//     Email,
//     Length(LengthArgs),
// }
//
// #[derive(Debug, Default, FromMeta)]
// #[darling(default)]
// struct LengthArgs {
//     min: Option<usize>,
//     max: Option<usize>,
// }

#[proc_macro_derive(Distilled, attributes(distilled))]
pub fn distilled_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(item as DeriveInput);

    let input = match Input::from_derive_input(&derive_input) {
        Ok(i) => i,
        Err(err) => return err.write_errors().into(),
    };

    let name = &input.ident;

    let (style, fields) = match input.data {
        Data::Struct(ds) => ds.split(),
        _ => unreachable!("Distilled only supports structs"),
    };

    let expanded = match style {
        Style::Struct => expand_struct(name, &fields),
        Style::Tuple if fields.len() == 1 => expand_newtype(name, &fields[0]),
        _ => unreachable!(),
    };

    expanded.into()
}

fn expand_struct(struct_ident: &syn::Ident, fields: &[Field]) -> proc_macro2::TokenStream {
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

fn expand_newtype(struct_ident: &syn::Ident, field: &Field) -> proc_macro2::TokenStream {
    let inner_ty = &field.ty;

    println!("EMAIL? {:?}", field.email);

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

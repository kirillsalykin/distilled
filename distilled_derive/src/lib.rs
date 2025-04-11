extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::quote;
use syn::Data;

#[proc_macro_derive(Distilled)]
pub fn distill_derive(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);

    let struct_ident = &input.ident;

    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let num_fields = fields.len();

            let distill_fields = fields.iter().map(|field| {
              let ident = field.ident.as_ref().unwrap();
              let ty = &field.ty;
              let name_str = ident.to_string();
              quote! {
                let #ident = <#ty>::distill_from(value.get(#name_str));
              }
            });

            let no_errors = fields.iter().map(|field| {
              let ident = field.ident.as_ref().unwrap();
              quote! {
                #ident.is_ok()
              }
            });

            let list_fields = fields.iter().map(|field| {
              let ident = field.ident.as_ref().unwrap();
              quote! {
                #ident: #ident.unwrap()
              }
            });

            let collect_errors = fields.iter().map(|field| {
              let ident = field.ident.as_ref().unwrap();
              let name_str = ident.to_string();
              quote! {
                if #ident.is_err() {
                  errors.insert(#name_str.into(), #ident.err().unwrap());
                }
              }
            });

            quote! {
                impl Distilled for #struct_ident {
                    fn distill_from<'a, T:std::convert::Into<std::option::Option<&'a serde_json::Value>>>(value: T) -> Result<Self, Error> {
                        let value = value.into().ok_or(Error::entry("missing_field"))?;

                        let mut errors = std::collections::HashMap::with_capacity(#num_fields);

                        #(#distill_fields)*

                        if #(#no_errors)&&* {
                          Ok(#struct_ident {
                           #(#list_fields),*
                          })
                        } else {
                          #(#collect_errors)*
                          Err(Error::Struct(errors))
                        }
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}

// struct SignUpInput {
//     field_string: String,
//     field_option: Option<u32>,
//     email: Email,
//     password: PlainTextPassword,
// }
//
// impl Distilled for SignUpInput {
//     fn distill_from<'a, T: Into<Option<&'a Value>>>(value: T) -> Result<Self, Error> {
//         let value = value.into().ok_or(Error::entry("missing_field"))?;
//
//         let mut errors = HashMap::with_capacity(4);
//
//         let field_string = String::distill_from(value.get("field_string"));
//         let field_option = Option::<u32>::distill_from(value.get("field_option"));
//         let email = Email::distill_from(value.get("email"));
//         let password = PlainTextPassword::distill_from(value.get("password"));
//
//         if field_string.is_ok() && field_option.is_ok() && email.is_ok() && password.is_ok() {
//             Ok(SignUpInput {
//                 field_string: field_string.unwrap(),
//                 field_option: field_option.unwrap(),
//                 email: email.unwrap(),
//                 password: password.unwrap(),
//             })
//         } else {
//             if field_string.is_err() {
//                 errors.insert("field_string".into(), field_string.err().unwrap());
//             }
//
//             if field_option.is_err() {
//                 errors.insert("field_option".into(), field_option.err().unwrap());
//             }
//
//             if email.is_err() {
//                 errors.insert("email".into(), email.err().unwrap());
//             }
//
//             if password.is_err() {
//                 errors.insert("password".into(), password.err().unwrap());
//             }
//
//             Err(Error::Struct(errors))
//         }
//     }
// }

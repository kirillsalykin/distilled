// TODO:
// Custom Xform/rule support
// [ ] Enum support
// [ ] Vec support

extern crate proc_macro2;

use darling::{
    FromDeriveInput, FromField, FromMeta,
    ast::{Data, Style},
};
use quote::{format_ident, quote};
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

    #[darling(default, multiple)]
    xforms: Vec<Xform>,

    #[darling(default, multiple)]
    rules: Vec<Rule>,
}

#[derive(Debug, FromMeta)]
#[darling(rename_all = "lowercase")]
enum Xform {
    Trim,
}

#[derive(Debug, FromMeta)]
#[darling(rename_all = "lowercase")]
enum Rule {
    Email,
    Length(LengthArgs),
}

#[derive(Debug, Default, FromMeta)]
#[darling(default)]
struct LengthArgs {
    min: Option<usize>,
    max: Option<usize>,
}

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
    let ty = &field.ty;

    let distilling_ident = format_ident!("__distilling");

    let xforms = build_xforms(&field.xforms, &distilling_ident);
    let rules = build_rules(&field.rules, &distilling_ident);

    quote! {
        impl ::distilled::Distilled for #struct_ident {
            fn distill<'a, T: Into<Option<&'a serde_json::Value>>>(
                value: T
            ) -> Result<Self, ::distilled::Error> {
                let mut #distilling_ident = <#ty>::distill(value)?;

                #(#xforms)*

                #(#rules)*

                Ok(#struct_ident(#distilling_ident))
            }
        }
    }
}

// XFORMS

fn build_xforms(xforms: &[Xform], ident: &syn::Ident) -> Vec<proc_macro2::TokenStream> {
    xforms.iter().map(|x| build_xform(x, ident)).collect()
}

fn build_xform(xform: &Xform, ident: &syn::Ident) -> proc_macro2::TokenStream {
    match xform {
        Xform::Trim => xform_trim(ident),
    }
}

fn xform_trim(ident: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        #ident = {
            let s: String = #ident.into();
            s.trim().to_string().into()
        };
    }
}

// RULES

fn build_rules(rules: &[Rule], ident: &syn::Ident) -> Vec<proc_macro2::TokenStream> {
    rules.iter().map(|x| build_rule(x, ident)).collect()
}

fn build_rule(rule: &Rule, ident: &syn::Ident) -> proc_macro2::TokenStream {
    match rule {
        Rule::Email => rule_email(ident),
        Rule::Length(args) => rule_length(args, ident),
        // add more Rule variants here…
    }
}

fn rule_email(ident: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        if !::validator::ValidateEmail::validate_email(&#ident) {
            return Err(::distilled::Error::entry("email"));
        }
    }
}

fn rule_length(args: &LengthArgs, ident: &syn::Ident) -> proc_macro2::TokenStream {
    let min_check = args.min.map(|min| {
        quote! {
            if #ident.len() < #min {
                return Err(::distilled::Error::entry("length"));
            }
        }
    });
    let max_check = args.max.map(|max| {
        quote! {
            if #ident.len() > #max {
                return Err(::distilled::Error::entry("length"));
            }
        }
    });

    quote! {
        #min_check
        #max_check
    }
}

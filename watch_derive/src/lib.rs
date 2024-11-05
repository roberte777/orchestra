extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Watchable)]
pub fn watchable_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let mut match_arms = Vec::new();

    if let syn::Data::Struct(data_struct) = data {
        if let syn::Fields::Named(fields_named) = data_struct.fields {
            for field in fields_named.named {
                if let Some(field_ident) = field.ident {
                    let field_name = field_ident.to_string();
                    match_arms.push(quote! {
                        #field_name => Some(self.#field_ident.to_string()),
                    });
                }
            }
        }
    }

    let gen = quote! {
        impl Watchable for #ident {
            fn get_field_value(&self, field_name: &str) -> Option<String> {
                match field_name {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }
    };

    gen.into()
}

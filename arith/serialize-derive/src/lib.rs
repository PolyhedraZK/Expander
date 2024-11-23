extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(FieldSerde)]
pub fn derive_field_serde_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;

    let named_fields = if let Data::Struct(data_struct) = input.data {
        if let Fields::Named(fields) = data_struct.fields {
            fields.named
        } else {
            panic!("FieldSerde can only be derived for structs with named fields.");
        }
    } else {
        panic!("FieldSerde can only be derived for structs.");
    };

    let serialize_fields_code_gen: Vec<_> = named_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();

            quote! {
                self.#field_name.serialize_into(&mut writer)?;
            }
        })
        .collect();

    let deserialize_fields_code_gen: Vec<_> = named_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_ty = &f.ty;

            quote! {
                let #field_name = <#field_ty as FieldSerde>::deserialize_from(&mut reader)?;
            }
        })
        .collect();

    let field_names_comma: Vec<_> = named_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();

            quote! {
                #field_name,
            }
        })
        .collect();

    let extended = quote! {
        impl FieldSerde for #struct_name {
            fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
                #(#serialize_fields_code_gen)*
                Ok(())
            }

            fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
                #(#deserialize_fields_code_gen)*

                Ok(Self {
                    #(#field_names_comma)*
                })
            }
        }
    };

    TokenStream::from(extended)
}

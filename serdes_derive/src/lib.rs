use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ExpSerde)]
pub fn serdes_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            Fields::Unnamed(fields) => fields.unnamed,
            Fields::Unit => return quote! {}.into(),
        },
        _ => return quote! {}.into(),
    };

    let field_names: Vec<_> = fields
        .iter()
        .map(|field| {
            field.ident.as_ref().unwrap_or_else(|| {
                panic!("Tuple structs are not supported by ExpSerde derive macro")
            })
        })
        .collect();

    let field_types: Vec<_> = fields.iter().map(|field| &field.ty).collect();

    let serialize_impl = quote! {
        fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
            #(
                self.#field_names.serialize_into(&mut writer)?;
            )*
            Ok(())
        }
    };

    let deserialize_impl = quote! {
        fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
            Ok(Self {
                #(
                    #field_names: <#field_types as serdes::ExpSerde>::deserialize_from(&mut reader)?,
                )*
            })
        }
    };

    let expanded = quote! {
        impl #impl_generics serdes::ExpSerde for #name #ty_generics #where_clause {

            #serialize_impl

            #deserialize_impl
        }
    };

    expanded.into()
}

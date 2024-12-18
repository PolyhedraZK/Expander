use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, ExprPath, Ident, Token};

// Define a struct to parse our custom input format
struct ConfigLit {
    config_name: Ident,
    field_expr: ExprPath,
    fiat_shamir_hash_type_expr: ExprPath,
    polynomial_commitment_type: ExprPath,
}

// Implement parsing for our custom input format
impl Parse for ConfigLit {
    fn parse(input: ParseStream) -> Result<Self> {
        let config_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let field_expr: ExprPath = input.parse()?;
        input.parse::<Token![,]>()?;
        let fiat_shamir_hash_type_expr: ExprPath = input.parse()?;
        input.parse::<Token![,]>()?;
        let polynomial_commitment_type: ExprPath = input.parse()?;
        let _ = input.parse::<Token![,]>(); // Optional trailing comma
        Ok(ConfigLit {
            config_name,
            field_expr,
            fiat_shamir_hash_type_expr,
            polynomial_commitment_type,
        })
    }
}

// Check if the field type is one of the supported types and return the corresponding config type
fn parse_field_type(field_expr: ExprPath) -> (String, String) {
    let field_enum = field_expr
        .path
        .segments
        .last()
        .expect("Empty path for field");
    match field_enum.ident.to_string().as_str() {
        "M31" => ("M31".to_owned(), "M31ExtConfig".to_owned()),
        "BN254" => ("BN254".to_owned(), "BN254Config".to_owned()),
        "GF2" => ("GF2".to_owned(), "GF2ExtConfig".to_owned()),
        _ => panic!("Unknown field type"),
    }
}

// Check if the hash type is one of the supported types and return the corresponding enum
fn parse_fiat_shamir_hash_type(
    field_config: &str,
    fiat_shamir_hash_type: ExprPath,
) -> (String, String) {
    let hash_enum = fiat_shamir_hash_type
        .path
        .segments
        .last()
        .expect("Empty path for hash type");

    let binding = hash_enum.ident.to_string();
    let hash_type_str = binding.as_str();
    let field_type = format!("<{field_config} as GKRFieldConfig>::ChallengeField");
    match hash_type_str {
        "SHA256" => (
            "SHA256".to_owned(),
            format!("BytesHashTranscript::<{field_type}, SHA256hasher>").to_owned(),
        ),
        "Keccak256" => (
            "Keccak256".to_owned(),
            format!("BytesHashTranscript::<{field_type}, Keccak256hasher>").to_owned(),
        ),
        "MIMC5" => (
            "MIMC5".to_owned(),
            format!("FieldHashTranscript::<{field_type}, MIMCHasher<{field_type}>>").to_owned(),
        ),
        _ => panic!("Unknown hash type"),
    }
}

fn parse_polynomial_commitment_type(
    field_type: &str,
    field_config: &str,
    transcript_type: &str,
    polynomial_commitment_type: ExprPath,
) -> (String, String) {
    let binding = polynomial_commitment_type
        .path
        .segments
        .last()
        .expect("Empty path for polynomial commitment type");

    let pcs_type_str = binding.ident.to_string();
    match (pcs_type_str.as_str(), field_type) {
        ("Raw", _) => (
            "Raw".to_owned(),
            format!("RawExpanderGKR::<{field_config}, {transcript_type}>").to_owned(),
        ),
        ("Orion", "GF2") => (
            "Orion".to_owned(),
            format!(
                "
OrionSIMDFieldPCS::<
    <{field_config} as GKRFieldConfig>::CircuitField,
    <{field_config} as GKRFieldConfig>::SimdCircuitField,
    <{field_config} as GKRFieldConfig>::ChallengeField,
    GF2x128,
    <{field_config} as GKRFieldConfig>::SimdCircuitField,
    {transcript_type}
>"
            )
            .to_owned(),
        ),
        _ => panic!("Unknown polynomial commitment type in config macro expansion"),
    }
}

/// Example usage:
/// declare_gkr_config!(
///     MyFavoriateConfigName,
///     FieldType::M31,
///     FiatShamirHashType::SHA256,
///     PolynomialCommitmentType::Raw
/// );
#[proc_macro]
pub fn declare_gkr_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_gkr_config_impl(input)
}

fn declare_gkr_config_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into our custom struct
    let ConfigLit {
        config_name,
        field_expr,
        fiat_shamir_hash_type_expr,
        polynomial_commitment_type,
    } = parse_macro_input!(input as ConfigLit);

    let (field_type, field_config) = parse_field_type(field_expr);
    let (fiat_shamir_hash_type, transcript_type) =
        parse_fiat_shamir_hash_type(field_config.as_str(), fiat_shamir_hash_type_expr);
    let (polynomial_commitment_enum, polynomial_commitment_type) = parse_polynomial_commitment_type(
        &field_type,
        &field_config,
        &transcript_type,
        polynomial_commitment_type,
    );

    let field_config = format_ident!("{field_config}");
    let fiat_shamir_hash_type = format_ident!("{fiat_shamir_hash_type}");
    let transcript_type_expr = syn::parse_str::<syn::Type>(&transcript_type).unwrap();
    let polynomial_commitment_enum = format_ident!("{polynomial_commitment_enum}");
    let polynomial_commitment_type_expr =
        syn::parse_str::<syn::Type>(&polynomial_commitment_type).unwrap();

    let ret: TokenStream = quote! {
        #[derive(Default, Debug, Clone, PartialEq)]
        struct #config_name;

        impl GKRConfig for #config_name {
            type FieldConfig = #field_config;
            const FIAT_SHAMIR_HASH: FiatShamirHashType = FiatShamirHashType::#fiat_shamir_hash_type;
            type Transcript = #transcript_type_expr;
            const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::#polynomial_commitment_enum;
            type PCS = #polynomial_commitment_type_expr;
        }
    };

    ret.into()
}

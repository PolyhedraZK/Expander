use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, ExprPath, Ident, Token, Visibility};

// Define a struct to parse our custom input format
struct ConfigLit {
    visibility: Visibility,
    config_name: Ident,
    field_expr: ExprPath,
    fiat_shamir_hash_type_expr: ExprPath,
    polynomial_commitment_type: ExprPath,
}

// Implement parsing for our custom input format
impl Parse for ConfigLit {
    fn parse(input: ParseStream) -> Result<Self> {
        let visibility: Visibility = input.parse()?;
        let config_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let field_expr: ExprPath = input.parse()?;
        input.parse::<Token![,]>()?;
        let fiat_shamir_hash_type_expr: ExprPath = input.parse()?;
        input.parse::<Token![,]>()?;
        let polynomial_commitment_type: ExprPath = input.parse()?;
        let _ = input.parse::<Token![,]>(); // Optional trailing comma
        Ok(ConfigLit {
            visibility,
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
    field_type: &str,
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
    let challenge_f = format!("<{field_config} as GKRFieldConfig>::ChallengeField");
    match (hash_type_str, field_type) {
        ("SHA256", _) => (
            "SHA256".to_owned(),
            format!("BytesHashTranscript::<{challenge_f}, SHA256hasher>").to_owned(),
        ),
        ("Keccak256", _) => (
            "Keccak256".to_owned(),
            format!("BytesHashTranscript::<{challenge_f}, Keccak256hasher>").to_owned(),
        ),
        ("Poseidon", "M31") => (
            "Poseidon".to_owned(),
            format!("FieldHashTranscript::<{challenge_f}, PoseidonFiatShamirHasher<M31x16>>")
                .to_owned(),
        ),
        ("MIMC5", "BN254") => (
            "MIMC5".to_owned(),
            format!("FieldHashTranscript::<{challenge_f}, MiMC5FiatShamirHasher<{challenge_f}>>")
                .to_owned(),
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
        ("Hyrax", "BN254") => (
            "Hyrax".to_owned(),
            format!("HyraxPCS::<G1Affine, {transcript_type}>").to_owned(),
        ),
        ("Orion", "GF2") => (
            "Orion".to_owned(),
            format!("OrionPCSForGKR::<{field_config}, GF2x128, {transcript_type}>").to_owned(),
        ),
        ("Orion", "M31") => (
            "Orion".to_owned(),
            format!("OrionPCSForGKR::<{field_config}, M31x16, {transcript_type}>").to_owned(),
        ),
        _ => panic!("Unknown polynomial commitment type in config macro expansion"),
    }
}

/// Example usage:
/// declare_gkr_config!(
///     pub MyFavoriateConfigName,
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
        visibility,
        config_name,
        field_expr,
        fiat_shamir_hash_type_expr,
        polynomial_commitment_type,
    } = parse_macro_input!(input as ConfigLit);

    let (field_type, field_config) = parse_field_type(field_expr);
    let (fiat_shamir_hash_type, transcript_type) =
        parse_fiat_shamir_hash_type(&field_type, &field_config, fiat_shamir_hash_type_expr);
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
        #visibility struct #config_name;

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

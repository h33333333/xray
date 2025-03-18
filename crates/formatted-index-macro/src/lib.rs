use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(FormattedIndex)]
pub fn formatted_index_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the enum
    let name = &input.ident;

    // Only work with enums
    let data_enum = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return quote! {
                compile_error!("FormattedIndex can only be derived for enums");
            }
            .into()
        }
    };

    // Generate match arms for each variant
    let variants = data_enum.variants.iter().enumerate().map(|(i, variant)| {
        let variant_name = &variant.ident;
        // Convert to 1-based index
        let one_based_index = i + 1;
        let formatted_index = format!("[{}]", one_based_index);

        quote! {
            #name::#variant_name { .. } => #formatted_index,
        }
    });

    // Generate the implementation
    let expanded = quote! {
        impl #name {
            /// Returns the variant's formatted index.
            pub const fn to_formatted_index(&self) -> &'static str {
                match self {
                    #(#variants)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

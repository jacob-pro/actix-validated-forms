extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(FromMultipart)]
pub fn impl_from_multipart(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let str = match &ast.data {
        syn::Data::Struct(s) => s,
        _ => panic!("This trait can only be derived for a struct"),
    };
    let fields = match &str.fields {
        syn::Fields::Named(n) => n,
        _ => panic!("This trait can only be derived for a struct"),
    };

    let mut fields_vec_innards = quote!();
    for field in fields.named.iter() {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        fields_vec_innards.extend(quote!(
            #name: 5,
        ));
    }

    let gen = quote! {
        impl std::convert::TryFrom<actix_validated_forms::multipart::MultipartForm> for #name {
            type Error = ();
            fn try_from(value: Vec<actix_validated_forms::multipart::MultipartField>) -> Result<Self, Self::Error> {
                let x = Self {
                    #fields_vec_innards
                };
                Ok(x)
            }
        }
    };
    gen.into()
}

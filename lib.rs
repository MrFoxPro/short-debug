use proc_macro2::TokenStream;
use quote::quote;
use syn::Fields;
use synstructure::{AddBounds, BindingInfo, Structure, VariantInfo, decl_derive};

decl_derive!([Debug, attributes(debug)] => custom_debug_derive);

fn custom_debug_derive(mut structure: Structure) -> TokenStream {
	structure.add_bounds(AddBounds::Fields);
	let match_arms = structure.each_variant(generate_match_arm_body);
	structure.gen_impl(quote! {
		gen impl ::core::fmt::Debug for @Self {
			fn fmt(&self, fmt: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				match *self {
					#match_arms
				}
			}
		}
	})
}

fn generate_match_arm_body(variant: &VariantInfo) -> TokenStream {
	let name = variant.ast().ident.to_string();
	let debug_builder = match variant.ast().fields {
		Fields::Named(_) | Fields::Unit => quote! { debug_struct },
		Fields::Unnamed(_) => quote! { debug_tuple },
	};
	let mut debug_builder_calls = Vec::new();
	for binding in variant.bindings() {
		debug_builder_calls.push(generate_debug_builder_call(binding));
	}
	quote! {
		let mut debug_builder = fmt.#debug_builder(#name);
		#(#debug_builder_calls)*
		debug_builder.finish()
	}
}

fn generate_debug_builder_call(binding: &BindingInfo) -> TokenStream {
	let format = quote! { #binding };

	let Some(name) = binding.ast().ident.as_ref().map(<_>::to_string) 
	else { return quote! { debug_builder.field(#format); } };

	if let syn::Type::Path(syn::TypePath { path: syn::Path { segments, .. }, .. }) = &binding.ast().ty {
		if segments.first().is_some_and(|seg| seg.ident.to_string() == "Option")  {
			return quote! { 
				if let Some(v) = #format { debug_builder.field(#name, v); }
			}
		}
		else if segments.first().is_some_and(|seg| seg.ident.to_string() == "Vec") {
			return quote! {
				if !#format.is_empty() { debug_builder.field(#name, #format); }
			}
		}
	}

	quote! { debug_builder.field(#name, #format); }
}

use proc_macro2::TokenStream;
use quote::quote;
use syn::Fields;
use synstructure::{decl_derive, AddBounds, BindingInfo, Structure, VariantInfo};

// custom `Debug`-like derive macro that does same thing as std::fmt::Debug
// but skips Option::None and Vec::is_empty fields
// and prints inner values of Option without Some(..) wrappers

decl_derive!([ShortDebug, attributes(debug)] => custom_debug_derive);

// Entry point of the derive macro implementation
fn custom_debug_derive(mut structure: Structure) -> TokenStream {
	// Add trait bounds to fields (e.g., require Debug on each field)
	structure.add_bounds(AddBounds::Fields);

	// Generate match arms for each enum variant or struct constructor
	let match_arms = structure.each_variant(generate_match_arm_body);

	// Generate full `impl Debug for T` block
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

// Generates the body of a match arm for a single variant (struct or enum)
fn generate_match_arm_body(variant: &VariantInfo) -> TokenStream {
	// Name of the variant or struct
	let name = variant.ast().ident.to_string();

	// Choose debug struct/tuple builder based on field style
	let debug_builder = match variant.ast().fields {
		Fields::Named(_) | Fields::Unit => quote! { debug_struct },
		Fields::Unnamed(_) => quote! { debug_tuple },
	};

	// Generate `.field(...)` or conditional field calls
	let mut debug_builder_calls = Vec::new();
	for binding in variant.bindings() {
		debug_builder_calls.push(generate_debug_builder_call(binding));
	}

	// Generate code like:
	// let mut debug_builder = fmt.debug_struct("VariantName");
	// debug_builder.field("field", value);
	// debug_builder.finish()
	quote! {
		let mut debug_builder = fmt.#debug_builder(#name);
		#(#debug_builder_calls)*
		debug_builder.finish()
	}
}

// Generates code for a single `.field(...)` call in the builder
fn generate_debug_builder_call(binding: &BindingInfo) -> TokenStream {
	let format = quote! { #binding };

	// Try to extract the field name, or fall back to unnamed field formatting
	let Some(name) = binding.ast().ident.as_ref().map(<_>::to_string)
	else {
		return quote! { debug_builder.field(#format); };
	};

	// Handle special-case field types: Option<T> and Vec<T>
	if let syn::Type::Path(syn::TypePath { path: syn::Path { segments, .. }, .. }) =
		&binding.ast().ty
	{
		if segments.first().is_some_and(|seg| seg.ident == "Option") {
			// Only print Some(...) fields
			return quote! {
				if let Some(v) = #format { debug_builder.field(#name, v); }
			};
		}
		else if segments.first().is_some_and(|seg| seg.ident == "Vec") {
			// Only print non-empty Vec fields
			return quote! {
				if !#format.is_empty() { debug_builder.field(#name, #format); }
			};
		}
	}

	// Default: always print the field
	quote! { debug_builder.field(#name, #format); }
}

#![warn(rustdoc::broken_intra_doc_links)]
//! Procedural macros for Drevo widgets.
//!
//! The widget derive delegates to a single field by default. Use
//! `#[widget_trait(field = name)]` to select one.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Member, Type, parse_macro_input, parse_quote};

/// Sets a child in a unique slot and returns it, assuming the slot manager is named `slots`. The
/// child initially shares its parent's hitbox. Its public `layer` defaults to zero and can be
/// changed before returning it from `layout`.
#[proc_macro]
pub fn display(input: TokenStream) -> TokenStream {
    let child = parse_macro_input!(input as Expr);

    quote! {
        slots.set(::drevo::num_id!(), #child).await?
    }
    .into()
}

#[proc_macro_derive(Style)]
/// Derives `.style(style) -> Self` for a struct with a `style: Style<...>` field.
///
/// This is only a bodge and allows nicer syntax.
pub fn derive_style(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_style(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_style(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Style can only be derived for structs",
        ));
    };

    let style_field = data
        .fields
        .iter()
        .find(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| ident == "style")
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            syn::Error::new_spanned(&input.ident, "Style derive requires a field named `style`")
        })?;

    let concrete_style = extract_concrete_style(&style_field.ty)?;
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        // This is only a bodge and allows nicer syntax.
        impl #impl_generics #name #ty_generics #where_clause {
            /// Sets the style of the widget.
            ///
            /// This is only a bodge and allows nicer syntax.
            pub fn style(mut self, style: #concrete_style) -> Self {
                self.style.set(style);
                self
            }
        }
    })
}

fn extract_concrete_style(ty: &Type) -> syn::Result<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Style" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(concrete_ty) = arg {
                            return Ok(concrete_ty);
                        }
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "expected field `style` to be of type `Style<...>`",
    ))
}

#[proc_macro_derive(WidgetTrait, attributes(widget_trait))]
/// Derives `drevo::widget::WidgetTrait` by delegating to a field.
pub fn derive_widget_trait(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_widget_trait(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_widget_trait(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let (field, field_type) = delegated_field(&input, "widget_trait", "WidgetTrait")?;
    let name = input.ident;
    let mut generics = input.generics;

    generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(#field_type: ::drevo::widget::WidgetTrait));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #[::async_trait::async_trait]
        impl #impl_generics ::drevo::widget::WidgetTrait
            for #name #ty_generics #where_clause
        {
            async fn layout(
                &mut self,
                input: ::drevo::widget::LayoutInput<'_>,
            ) -> ::color_eyre::eyre::Result<::drevo::component::Children> {
                ::drevo::widget::WidgetTrait::layout(
                    &mut self.#field,
                    input,
                )
                .await
            }

            async fn render(
                &mut self,
                input: ::drevo::widget::RenderInput<'_, '_>,
            ) -> ::color_eyre::eyre::Result<()> {
                ::drevo::widget::WidgetTrait::render(
                    &mut self.#field,
                    input,
                )
                .await
            }

            async fn on_all_events(
                &mut self,
                input: ::drevo::widget::AllEvents<'_>,
            ) -> ::color_eyre::eyre::Result<::drevo::DrevoMsg> {
                ::drevo::widget::WidgetTrait::on_all_events(&mut self.#field, input).await
            }

            async fn on_mouse_click(
                &mut self,
                input: ::drevo::widget::MouseEvent<'_>,
            ) -> ::color_eyre::eyre::Result<::drevo::DrevoMsg> {
                ::drevo::widget::WidgetTrait::on_mouse_click(&mut self.#field, input).await
            }

            async fn on_key_press(
                &mut self,
                input: ::drevo::widget::KeyPress<'_>,
            ) -> ::color_eyre::eyre::Result<::drevo::DrevoMsg> {
                ::drevo::widget::WidgetTrait::on_key_press(&mut self.#field, input).await
            }

            async fn on_other_event(
                &mut self,
                input: ::drevo::widget::OtherEvent<'_>,
            ) -> ::color_eyre::eyre::Result<::drevo::DrevoMsg> {
                ::drevo::widget::WidgetTrait::on_other_event(&mut self.#field, input).await
            }

            async fn forward_event(
                &mut self,
                event: &::drevo::event::Event,
                relayout: ::drevo::Signal,
                window: ::std::option::Option<::std::sync::Arc<::drevo::Window>>,
            ) -> ::color_eyre::eyre::Result<::drevo::DrevoMsg> {
                ::drevo::widget::WidgetTrait::forward_event(&mut self.#field, event, relayout, window).await
            }
        }
    })
}

fn delegated_field(
    input: &DeriveInput,
    attribute_name: &str,
    derive_name: &str,
) -> syn::Result<(Member, Type)> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("{derive_name} can only be derived for structs"),
        ));
    };

    let configured_field = input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident(attribute_name))
        .map(|attribute| {
            let mut field = None;

            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("field") {
                    if field.is_some() {
                        return Err(meta.error("field can only be specified once"));
                    }

                    field = Some(meta.value()?.parse::<Member>()?);
                    Ok(())
                } else {
                    Err(meta.error("unsupported attribute"))
                }
            })?;

            field.ok_or_else(|| {
                syn::Error::new_spanned(
                    attribute,
                    format!("expected #[{attribute_name}(field = field_name)]"),
                )
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let member = match configured_field.as_slice() {
        [] => single_field_member(&data.fields, attribute_name, derive_name)?,
        [field] => field.clone(),
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                format!("{attribute_name} can only be specified once"),
            ));
        }
    };
    let field_type = field_type(&data.fields, &member, attribute_name)?;

    Ok((member, field_type.clone()))
}

fn single_field_member(
    fields: &Fields,
    attribute_name: &str,
    derive_name: &str,
) -> syn::Result<Member> {
    match fields {
        Fields::Named(fields) if fields.named.len() == 1 => {
            Ok(Member::Named(fields.named[0].ident.clone().unwrap()))
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(Member::Unnamed(0.into())),
        _ => Err(syn::Error::new_spanned(
            fields,
            format!(
                "{derive_name} requires #[{attribute_name}(field = field_name)] \
                 unless the struct has exactly one field"
            ),
        )),
    }
}

fn field_type<'a>(
    fields: &'a Fields,
    member: &Member,
    attribute_name: &str,
) -> syn::Result<&'a Type> {
    let field = match (fields, member) {
        (Fields::Named(fields), Member::Named(name)) => fields
            .named
            .iter()
            .find(|field| field.ident.as_ref() == Some(name)),
        (Fields::Unnamed(fields), Member::Unnamed(index)) => {
            fields.unnamed.iter().nth(index.index as usize)
        }
        _ => None,
    };

    field.map(|field| &field.ty).ok_or_else(|| {
        syn::Error::new_spanned(
            member,
            format!("unknown field specified by #[{attribute_name}(field = ...)]"),
        )
    })
}

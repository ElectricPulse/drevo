#![warn(rustdoc::broken_intra_doc_links)]
//! Procedural macros for Vizual widgets.
//!
//! The widget derive delegates to a single field by default. Use
//! `#[widget_trait(field = name)]` to select one.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Member, Type, parse_macro_input, parse_quote};

/// Sets a child in a unique slot and returns it, assuming the slot manager is named `slots`.
#[proc_macro]
pub fn display(input: TokenStream) -> TokenStream {
    let child = parse_macro_input!(input as Expr);

    quote! {
        slots.set(::vizual::id!(), #child).await?
    }
    .into()
}

#[proc_macro_derive(Widget_trait, attributes(widget_trait))]
/// Derives `vizual::widget::Widget_trait` by delegating to a field.
pub fn derive_widget_trait(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_widget_trait(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_widget_trait(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let (field, field_type) = delegated_field(&input, "widget_trait", "Widget_trait")?;
    let name = input.ident;
    let mut generics = input.generics;

    generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(#field_type: ::vizual::widget::Widget_trait));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #[::async_trait::async_trait]
        impl #impl_generics ::vizual::widget::Widget_trait
            for #name #ty_generics #where_clause
        {
            async fn layout(
                &mut self,
                focus: &mut ::vizual::widget::Focus_provider,
                hitbox: &mut ::vizual::layouter::hitbox::Hitbox,
                parent: ::vizual::layouter::hitbox::Hitbox,
                problem: ::vizual::component::context::Component_context,
                text_context: &mut ::vizual::text::Text_context,
                slots: &mut ::vizual::slot::manager::Slots,
            ) -> ::color_eyre::eyre::Result<::vizual::component::Children> {
                ::vizual::widget::Widget_trait::layout(
                    &mut self.#field,
                    focus,
                    hitbox,
                    parent,
                    problem,
                    text_context,
                    slots,
                )
                .await
            }

            async fn render(
                &mut self,
                focus: &mut ::vizual::widget::Focus_provider,
                hitbox: ::vizual::geometry::Rect,
                display: &mut ::vizual::display::Display<'_>,
            ) -> ::color_eyre::eyre::Result<::std::option::Option<::vizual::layouter::hitbox::Hitbox>> {
                ::vizual::widget::Widget_trait::render(
                    &mut self.#field,
                    focus,
                    hitbox,
                    display,
                )
                .await
            }

            async fn on_all_events(
                &mut self,
                event: &::vizual::event::Event,
            ) -> ::color_eyre::eyre::Result<::vizual::Vizual_msg> {
                ::vizual::widget::Widget_trait::on_all_events(&mut self.#field, event).await
            }

            async fn on_mouse_click(
                &mut self,
                mouse: &::vizual::event::Pointer_event,
            ) -> ::color_eyre::eyre::Result<::vizual::Vizual_msg> {
                ::vizual::widget::Widget_trait::on_mouse_click(&mut self.#field, mouse).await
            }

            async fn on_key_press(
                &mut self,
                key: &::vizual::event::Key_event,
            ) -> ::color_eyre::eyre::Result<::vizual::Vizual_msg> {
                ::vizual::widget::Widget_trait::on_key_press(&mut self.#field, key).await
            }

            async fn on_other_event(
                &mut self,
                event: &::vizual::event::Event,
            ) -> ::color_eyre::eyre::Result<::vizual::Vizual_msg> {
                ::vizual::widget::Widget_trait::on_other_event(&mut self.#field, event).await
            }

            async fn forward_event(
                &mut self,
                event: &::vizual::event::Event,
            ) -> ::color_eyre::eyre::Result<::vizual::Vizual_msg> {
                ::vizual::widget::Widget_trait::forward_event(&mut self.#field, event).await
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

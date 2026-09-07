use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Children, IntoComponent},
    slot::manager::Slots,
};

/// Mounts a heterogeneous collection of children into consecutive slots.
///
/// Layout containers accept this trait instead of a collection of widgets so a child can be a
/// normal widget or an existing [`crate::component::SharedComponent`]. Slots are assigned in
/// collection order, beginning at zero.
#[async_trait]
pub trait IntoComponents: Send + Sync + dyn_clone::DynClone {
    async fn into_components(&self, slots: &mut Slots<'_>) -> Result<Children>;
}

dyn_clone::clone_trait_object!(IntoComponents);

pub type Components = Box<dyn IntoComponents>;

#[async_trait]
impl IntoComponents for () {
    async fn into_components(&self, _slots: &mut Slots<'_>) -> Result<Children> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl<Component> IntoComponents for Vec<Component>
where
    Component: IntoComponent + Clone,
{
    async fn into_components(&self, slots: &mut Slots<'_>) -> Result<Children> {
        let mut children = Vec::with_capacity(self.len());
        for (id, component) in self.iter().enumerate() {
            children.push(slots.set(id as u64, component.clone()).await?);
        }
        Ok(children)
    }
}

async fn mount<Component>(
    slots: &mut Slots<'_>,
    id: &mut u64,
    component: &Component,
) -> Result<crate::component::SharedComponent>
where
    Component: IntoComponent + Clone,
{
    let current_id = *id;
    *id += 1;
    slots.set(current_id, component.clone()).await
}

macro_rules! impl_into_components_for_tuples {
    ($(($($Component:ident),+ $(,)?))+) => {
        $(
            #[async_trait]
            #[allow(non_snake_case)]
            impl<$($Component),+> IntoComponents for ($($Component,)+)
            where
                $($Component: IntoComponent + Clone,)+
            {
                async fn into_components(&self, slots: &mut Slots<'_>) -> Result<Children> {
                    let ($($Component,)+) = self;
                    let mut id = 0_u64;
                    let mut children = Vec::new();
                    $(
                        children.push(mount(slots, &mut id, $Component).await?);
                    )+
                    Ok(children)
                }
            }
        )+
    };
}

impl_into_components_for_tuples! {
    (Component1,)
    (Component1, Component2)
    (Component1, Component2, Component3)
    (Component1, Component2, Component3, Component4)
    (Component1, Component2, Component3, Component4, Component5)
    (Component1, Component2, Component3, Component4, Component5, Component6)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11, Component12)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11, Component12, Component13)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11, Component12, Component13, Component14)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11, Component12, Component13, Component14, Component15)
    (Component1, Component2, Component3, Component4, Component5, Component6, Component7, Component8, Component9, Component10, Component11, Component12, Component13, Component14, Component15, Component16)
}

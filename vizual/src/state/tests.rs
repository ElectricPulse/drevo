use super::*;
use crate::render_manager::Render_manager;

struct Not_clone(u8);

#[tokio::test]
async fn store_clone_only_clones_the_arc() -> Result<()> {
    let store = Store::new(Not_clone(1));
    let cloned = store.clone();

    cloned.write().await?.0 = 2;

    assert_eq!(store.read().await?.0, 2);
    Ok(())
}

#[tokio::test]
async fn affect_deduplicates_render_ids_and_write_notifies_after_drop() -> Result<()> {
    let mut first_manager = Render_manager::new();
    let mut second_manager = Render_manager::new();
    assert_ne!(first_manager.render.id, second_manager.render.id);
    assert_eq!(first_manager.render.id, first_manager.render.clone().id);

    let store = Store::new(1_u8);
    drop(store.read().await?);
    assert!(first_manager.reciever.0.try_recv().is_err());

    drop(store.affect(first_manager.render.clone()).await?);
    drop(store.affect(first_manager.render.clone()).await?);
    drop(store.affect(second_manager.render.clone()).await?);

    let mut value = store.write().await?;
    *value = 2;
    assert!(first_manager.reciever.0.try_recv().is_err());
    assert!(second_manager.reciever.0.try_recv().is_err());
    drop(value);

    assert_eq!(first_manager.reciever.0.recv().await, Some(()));
    assert_eq!(second_manager.reciever.0.recv().await, Some(()));
    assert!(first_manager.reciever.0.try_recv().is_err());
    assert!(second_manager.reciever.0.try_recv().is_err());
    assert_eq!(*store.read().await?, 2);
    Ok(())
}

#[tokio::test]
async fn constant_never_subscribes() -> Result<()> {
    let mut manager = Render_manager::new();
    let constant = Constant::from(String::from("constant"));

    assert_eq!(&*constant.read().await?, "constant");
    assert_eq!(&*constant.affect(manager.render.clone()).await?, "constant");
    assert!(manager.reciever.0.try_recv().is_err());
    Ok(())
}

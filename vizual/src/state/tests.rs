use super::*;
use crate::render_manager::Render_manager;

#[tokio::test]
async fn store_clone_only_clones_the_arc() -> Result<()> {
    let store = Store::new(1_u8);
    let cloned = store.clone();

    cloned.set(2_u8).await?;

    assert_eq!(*store.get().await?, 2);
    Ok(())
}

#[tokio::test]
async fn affect_deduplicates_render_ids_and_set_notifies() -> Result<()> {
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

    store.set(2_u8).await?;

    assert_eq!(first_manager.reciever.0.recv().await, Some(()));
    assert_eq!(second_manager.reciever.0.recv().await, Some(()));
    assert!(first_manager.reciever.0.try_recv().is_err());
    assert!(second_manager.reciever.0.try_recv().is_err());
    assert_eq!(*store.get().await?, 2);
    Ok(())
}

#[tokio::test]
async fn store_set_another_store_forwards_notifications() -> Result<()> {
    let mut manager = Render_manager::new();
    let parent = Store::new(1_u8);
    drop(parent.affect(manager.render.clone()).await?);

    let child = Store::new(10_u8);
    parent.set(child.clone()).await?;

    // Parent notification on set
    assert_eq!(manager.reciever.0.recv().await, Some(()));

    // Changing child forwards to parent's subscriber
    child.set(20_u8).await?;
    assert_eq!(manager.reciever.0.recv().await, Some(()));
    assert_eq!(*parent.get().await?, 20);

    // Overwriting parent stops forwarding from old child
    parent.set(99_u8).await?;
    assert_eq!(manager.reciever.0.recv().await, Some(()));
    assert_eq!(*parent.get().await?, 99);

    // Further changes to old child do not affect parent
    child.set(30_u8).await?;
    assert_eq!(*parent.get().await?, 99);
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

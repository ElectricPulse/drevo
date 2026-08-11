use std::collections::HashMap;
use std::panic::Location;

use color_eyre::eyre::{Result, eyre};

use crate::{
    component::{Shared_component, context::Component_context},
    layouter::hitbox::Hitbox,
    widget::Widget_trait,
};

use super::Component_slot;

pub struct Record {
    pub id: Component_slot,
    pub mounted: bool,
}

// In the future this could also be passed into widget event methods to allow things like Focus to work.
pub struct Slot_records {
    // We store a reference to Component_context so that one doesn't have to pass it in set()
    pub(crate) problem: Component_context,
    records: HashMap<u64, Record>,
}

pub struct Slots<'a> {
    slot_manager: &'a mut Slot_records,
    parent: Hitbox,
    used: HashMap<u64, bool>,
    used_at: HashMap<u64, &'static Location<'static>>,
}

// Compatibility for Widget_trait derive output.
pub type Slot_manager<'a> = Slots<'a>;

impl Slot_records {
    pub fn new(problem: Component_context) -> Self {
        Self {
            records: HashMap::<u64, Record>::default(),
            problem,
        }
    }

    pub(crate) fn set_problem(&mut self, problem: Component_context) {
        self.problem = problem;
    }

    pub fn slots(&mut self, parent: &Hitbox) -> Slots<'_> {
        Slots {
            slot_manager: self,
            parent: parent.clone(),
            used: HashMap::new(),
            used_at: HashMap::new(),
        }
    }

    fn get_at(&mut self, id: u64, location: &'static Location<'static>) -> &mut Component_slot {
        let record = self.records.entry(id).or_insert_with(|| {
            // This is where on_mount could be implemented for a widget.
            Record {
                id: Component_slot::new_at(location),
                mounted: true,
            }
        });

        record.mounted = true;

        &mut record.id
    }

    pub async fn evaluate(&mut self) -> Result<()> {
        let dismounted = self
            .records
            .iter()
            .filter_map(|(id, record)| match record.mounted {
                true => None,
                false => Some(*id),
            })
            .collect::<Vec<_>>();

        for id in dismounted {
            if let Some(mut record) = self.records.remove(&id) {
                record.id.dismount().await?;
            }
        }

        for record in self.records.values_mut() {
            record.mounted = false;
        }

        Ok(())
    }
}

impl Slots<'_> {
    fn mark_used(&mut self, id: u64, location: &'static Location<'static>) -> Result<()> {
        match self.used.insert(id, true) {
            Some(true) => {
                let first = self.used_at[&id];
                Err(eyre!(
                    "slot {id} was already set at {}:{} and cannot be set again at {}:{}",
                    first.file(),
                    first.line(),
                    location.file(),
                    location.line(),
                ))
            }
            Some(false) | None => {
                let _ = self.used_at.insert(id, location);
                Ok(())
            }
        }
    }

    #[track_caller]
    pub async fn set(&mut self, id: u64, widget: impl Widget_trait) -> Result<Shared_component> {
        let location = Location::caller();
        self.mark_used(id, location)?;
        let problem = self.slot_manager.problem.clone();
        self.slot_manager
            .get_at(id, location)
            .set_child(widget, problem, &self.parent)
            .await
    }
}

#[macro_export]
macro_rules! id {
    () => {
        // TODO: This u64::MAX/2 offset is a workaround for namespace conflicts with other `slots.set` calls that
        // commonly use `set_generic` with the index from an iteration as the key.
        (::uniqify::uniqify!() as u64) + u64::MAX / 2
    };
}

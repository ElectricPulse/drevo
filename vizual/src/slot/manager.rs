use std::collections::HashMap;
use std::panic::Location;

use color_eyre::eyre::{Result, eyre};

use crate::{component::Shared_component, layouter::Problem_context, widget::Widget_trait};

use super::Component_slot;

pub struct Record {
    pub id: Component_slot,
    pub mounted: bool,
}

// In the future this could also be passed into Control methods to allow things like Focus to work.
pub struct Slot_records {
    // We store a reference to Problem_context so that one doesn't have to pass it in set()
    pub(crate) problem: Problem_context,
    records: HashMap<u64, Record>,
}

pub struct Slots<'a> {
    slot_manager: &'a mut Slot_records,
    used: HashMap<u64, bool>,
    used_at: HashMap<u64, &'static Location<'static>>,
}

// Compatibility for Widget_trait derive output.
pub type Slot_manager<'a> = Slots<'a>;

impl Slot_records {
    pub fn new(problem: Problem_context) -> Self {
        Self {
            records: HashMap::<u64, Record>::default(),
            problem,
        }
    }

    pub(crate) fn set_problem(&mut self, problem: Problem_context) {
        self.problem = problem;
    }

    pub fn slots(&mut self) -> Slots<'_> {
        Slots {
            slot_manager: self,
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

    pub fn evaluate(&mut self) {
        self.records.retain(|_, record| {
            if !record.mounted {
                // This is where on_unmount() could happen.
                return false;
            }

            record.mounted = false;
            true
        });
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
            .set(widget, problem)
            .await
    }
}

#[macro_export]
macro_rules! id {
    () => {
        // TODO: This u64::MAX/2 is a bodge to solve namespace conflicts with other `slots.set` calls that
        // commonly use `set_generic` with the index from an iteration as the key.
        (::uniqify::uniqify!() as u64) + u64::MAX / 2
    };
}

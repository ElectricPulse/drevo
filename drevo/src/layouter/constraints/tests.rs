use std::sync::Arc;

use color_eyre::eyre::Result;

use super::prohibit_overlap;
use crate::layouter::{Formula, hitbox::Hitbox, variables::Variables};

#[test]
fn prohibit_overlap_uses_two_binary_variables() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let first = Hitbox::new(
        &variables,
        "first".to_string(),
        "test".to_string(),
        "test".to_string(),
    );
    let second = Hitbox::new(
        &variables,
        "second".to_string(),
        "test".to_string(),
        "test".to_string(),
    );
    let mut formula = Formula::new(variables);

    prohibit_overlap(&mut formula, first, second, 8.0)?;

    assert_eq!(formula.variables.len(), 2);
    assert_eq!(formula.constraints.len(), 4);
    Ok(())
}

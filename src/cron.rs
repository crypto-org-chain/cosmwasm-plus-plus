use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitset::BitSet;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CronCompiled {
    pub minute: BitSet,
    pub hour: BitSet,
    pub mday: BitSet,
    pub month: BitSet,
    pub wday: BitSet,
}

impl CronCompiled {
    /// Return the smallest time matches cron spec and bigger than or equal to the input.
    pub fn round_up(&self, _time: NaiveDateTime) -> NaiveDateTime {
        unimplemented!()
    }
}

//! Parse and compile crontab syntax, not needed for on-chain code.
use std::str::FromStr;

use crate::bitset::{BitSet, BitSetIndex};
use crate::cron::CronCompiled;

pub const FULL_CRON: CronCompiled = CronCompiled {
    minute: BitSet::from_range(0, 59),
    hour: BitSet::from_range(0, 23),
    mday: BitSet::from_range(1, 31),
    wday: BitSet::from_range(0, 6),
    month: BitSet::from_range(1, 12),
};

#[derive(Clone, PartialEq, Debug)]
pub enum CronItem {
    Range {
        start: BitSetIndex,
        end: BitSetIndex,
        step: BitSetIndex,
    },
    Value(BitSetIndex),
}

impl CronItem {
    pub fn compile(&self) -> BitSet {
        let mut set = BitSet::new();
        match self {
            Self::Value(value) => set.set(*value),
            Self::Range { start, end, step } => {
                for idx in (start.value()..end.value()).step_by(step.value()) {
                    // SAFETY: idx < end.0 < 64
                    set.set(BitSetIndex::unsafe_new(idx as u8))
                }
            }
        };
        set
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CronError {
    Empty,
    OutOfRange,
}

/// Empty Vec means `*`
#[derive(Clone, PartialEq, Debug, Default)]
pub struct CronSpec {
    pub minute: Vec<CronItem>,
    pub hour: Vec<CronItem>,
    pub mday: Vec<CronItem>,
    pub month: Vec<CronItem>,
    pub wday: Vec<CronItem>,
}

impl CronSpec {
    pub fn compile(&self) -> Result<CronCompiled, CronError> {
        let minute = compile_component(&self.minute, FULL_CRON.minute);
        if let Some(max) = minute.max() {
            if max.value() > 59 {
                return Err(CronError::OutOfRange);
            }
        } else {
            return Err(CronError::Empty);
        }

        let hour = compile_component(&self.hour, FULL_CRON.hour);
        if let Some(max) = hour.max() {
            if max.value() > 23 {
                return Err(CronError::OutOfRange);
            }
        } else {
            return Err(CronError::Empty);
        }

        let mday = compile_component(&self.mday, FULL_CRON.mday);
        if let Some((min, max)) = mday.bound() {
            if max.value() > 31 {
                return Err(CronError::OutOfRange);
            }
            if min.value() < 1 {
                return Err(CronError::OutOfRange);
            }
        } else {
            return Err(CronError::Empty);
        }

        let month = compile_component(&self.month, FULL_CRON.month);
        if let Some((min, max)) = month.bound() {
            if max.value() > 12 {
                return Err(CronError::OutOfRange);
            }
            if min.value() < 1 {
                return Err(CronError::OutOfRange);
            }
        } else {
            return Err(CronError::Empty);
        }

        let wday = compile_component(&self.wday, FULL_CRON.wday);
        if let Some(max) = wday.max() {
            if max.value() > 6 {
                return Err(CronError::OutOfRange);
            }
        } else {
            return Err(CronError::Empty);
        }

        Ok(CronCompiled {
            minute,
            hour,
            mday,
            month,
            wday,
        })
    }
}

fn compile_component(items: &[CronItem], full: BitSet) -> BitSet {
    if items.is_empty() {
        full
    } else {
        let mut set = BitSet::new();
        for item in items.iter() {
            set.inplace_union(item.compile());
        }
        set
    }
}

impl FromStr for CronSpec {
    type Err = &'static str;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = v.split(' ').collect();
        if parts.len() != 5 {
            return Err("wrong number of cron components");
        }

        Ok(CronSpec {
            minute: parse_component(parts[0])?,
            hour: parse_component(parts[1])?,
            mday: parse_component(parts[2])?,
            month: parse_component(parts[3])?,
            wday: parse_component(parts[4])?,
        })
    }
}

fn parse_component(v: &str) -> Result<Vec<CronItem>, &'static str> {
    let mut result = Vec::new();
    for item in v.split(',') {
        let parts: Vec<&str> = item.split('-').collect();
        if parts.len() == 1 {
            if parts[0] == "*" {
                // any
                return Ok(vec![]);
            } else {
                // value
                result.push(CronItem::Value(parse_number(parts[0])?));
            }
        } else if parts.len() >= 2 && parts.len() <= 3 {
            // range
            result.push(CronItem::Range {
                start: parse_number(parts[0])?,
                end: parse_number(parts[1])?,
                step: parts
                    .get(2)
                    .map_or_else(|| Ok(BitSetIndex::unsafe_new(1)), |s| parse_number(*s))?,
            });
        } else {
            return Err("wrong range syntax in cron");
        }
    }
    Ok(result)
}

fn parse_number(v: &str) -> Result<BitSetIndex, &'static str> {
    let n = v.parse::<usize>().map_err(|_| "invalid cron number")?;
    BitSetIndex::new(n).ok_or("cron number out of range")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_compile() {
        let full = "* * * * *".parse::<CronSpec>().unwrap();
        assert_eq!(full, CronSpec::default());
        assert_eq!(full.compile().unwrap(), FULL_CRON);

        let wrong = "2-1 * * * *".parse::<CronSpec>().unwrap();
        assert_eq!(wrong.compile().unwrap_err(), CronError::Empty);
    }
}

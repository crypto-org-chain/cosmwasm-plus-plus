//! Parse and compile crontab syntax, not needed for on-chain code.
use std::str::FromStr;

use crate::bitset::{BitSetIndex, NonEmptyBitSet};
use crate::cron::CronCompiled;

#[derive(Clone, PartialEq, Debug)]
pub enum CronItem {
    Range {
        /// inclusive
        start: BitSetIndex,
        /// inclusive
        end: BitSetIndex,
        step: BitSetIndex,
    },
    Value(BitSetIndex),
}

impl CronItem {
    pub fn compile(&self) -> Option<NonEmptyBitSet> {
        match self {
            Self::Value(value) => Some(NonEmptyBitSet::new(*value)),
            Self::Range { start, end, step } => {
                NonEmptyBitSet::from_items((start.get()..=end.get()).step_by(step.get()))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CronError {
    Empty,
    OutOfRange,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CronSpec {
    pub minute: Vec<CronItem>,
    pub hour: Vec<CronItem>,
    pub mday: Vec<CronItem>,
    pub month: Vec<CronItem>,
    pub wday: Vec<CronItem>,
}

impl CronSpec {
    pub fn compile(&self) -> Result<CronCompiled, CronError> {
        let minute = compile_component(&self.minute).ok_or(CronError::Empty)?;
        if minute.max().get() > 59 {
            return Err(CronError::OutOfRange);
        }

        let hour = compile_component(&self.hour).ok_or(CronError::Empty)?;
        if hour.max().get() > 23 {
            return Err(CronError::OutOfRange);
        }

        let mday = compile_component(&self.mday).ok_or(CronError::Empty)?;
        if mday.max().get() > 31 {
            return Err(CronError::OutOfRange);
        }
        if mday.min().get() < 1 {
            return Err(CronError::OutOfRange);
        }

        let month = compile_component(&self.month).ok_or(CronError::Empty)?;
        if month.max().get() > 12 {
            return Err(CronError::OutOfRange);
        }
        if month.min().get() < 1 {
            return Err(CronError::OutOfRange);
        }

        let wday = compile_component(&self.wday).ok_or(CronError::Empty)?;
        if wday.max().get() > 6 {
            return Err(CronError::OutOfRange);
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

fn compile_component(items: &[CronItem]) -> Option<NonEmptyBitSet> {
    NonEmptyBitSet::from_bitsets(items.iter().map(|item| item.compile()).flatten())
}

impl FromStr for CronSpec {
    type Err = &'static str;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = v.split(' ').collect();
        if parts.len() != 5 {
            return Err("wrong number of cron components");
        }

        Ok(CronSpec {
            minute: parse_component(
                parts[0],
                BitSetIndex::new(0).unwrap(),
                BitSetIndex::new(59).unwrap(),
            )?,
            hour: parse_component(
                parts[1],
                BitSetIndex::new(0).unwrap(),
                BitSetIndex::new(23).unwrap(),
            )?,
            mday: parse_component(
                parts[2],
                BitSetIndex::new(1).unwrap(),
                BitSetIndex::new(31).unwrap(),
            )?,
            month: parse_component(
                parts[3],
                BitSetIndex::new(1).unwrap(),
                BitSetIndex::new(12).unwrap(),
            )?,
            wday: parse_component(
                parts[4],
                BitSetIndex::new(0).unwrap(),
                BitSetIndex::new(6).unwrap(),
            )?,
        })
    }
}

fn parse_component(
    v: &str,
    min: BitSetIndex,
    max: BitSetIndex,
) -> Result<Vec<CronItem>, &'static str> {
    let mut result = Vec::new();
    for item in v.split(',') {
        let parts: Vec<&str> = item.split('/').collect();
        if parts.len() == 1 || parts.len() == 2 {
            let step = parse_number(parts.get(1).unwrap_or(&"1"))?;
            let parts: Vec<&str> = parts[0].split('-').collect();
            if parts.len() == 1 {
                if parts[0] == "*" {
                    // any
                    result.push(CronItem::Range {
                        start: min,
                        end: max,
                        step,
                    });
                } else {
                    // value
                    result.push(CronItem::Value(parse_number(parts[0])?));
                }
            } else if parts.len() == 2 {
                // range
                result.push(CronItem::Range {
                    start: parse_number(parts[0])?,
                    end: parse_number(parts[1])?,
                    step,
                });
            } else {
                return Err("wrong range syntax in cron");
            }
        } else {
            return Err("wrong step syntax in cron");
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
        const FULL_CRON: CronCompiled = CronCompiled {
            minute: NonEmptyBitSet::from_range(0, 59),
            hour: NonEmptyBitSet::from_range(0, 23),
            mday: NonEmptyBitSet::from_range(1, 31),
            wday: NonEmptyBitSet::from_range(0, 6),
            month: NonEmptyBitSet::from_range(1, 12),
        };

        let full = "* * * * *".parse::<CronSpec>().unwrap();
        assert_eq!(full.compile().unwrap(), FULL_CRON);

        let empty = "2-1 * * * *".parse::<CronSpec>().unwrap();
        assert_eq!(empty.compile().unwrap_err(), CronError::Empty);

        let steps = "*/2,*/3 1-10/3 * * *".parse::<CronSpec>().unwrap();
        steps.compile().unwrap();
    }
}

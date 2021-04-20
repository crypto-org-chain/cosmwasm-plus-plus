use chrono::{Datelike, NaiveDateTime, Timelike};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitset::{BitSetIndex, NonEmptyBitSet};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CronCompiled {
    pub minute: NonEmptyBitSet,
    pub hour: NonEmptyBitSet,
    pub mday: NonEmptyBitSet,
    pub month: NonEmptyBitSet,
    pub wday: NonEmptyBitSet,
}

impl CronCompiled {
    /// Verify the datetime matches cron spec
    pub fn verify(&self, datetime: NaiveDateTime) -> bool {
        let time = datetime.time();
        let date = datetime.date();
        // SAFETY: range of value is guaranteed
        let minute = BitSetIndex::new(time.minute() as usize).unwrap();
        // SAFETY: range of value is guaranteed
        let hour = BitSetIndex::new(time.hour() as usize).unwrap();
        // SAFETY: range of value is guaranteed
        let mday = BitSetIndex::new(date.day() as usize).unwrap();
        // SAFETY: range of value is guaranteed
        let month = BitSetIndex::new(date.month() as usize).unwrap();
        // SAFETY: range of value is guaranteed
        let wday = BitSetIndex::new(date.weekday().num_days_from_sunday() as usize).unwrap();
        self.minute.test(minute)
            && self.hour.test(hour)
            && self.mday.test(mday)
            && self.month.test(month)
            && self.wday.test(wday)
            && time.second() == 0
            && time.nanosecond() == 0
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDate;

    use crate::cron_spec::CronSpec;

    #[test]
    fn cron_verificaton() {
        let cron = CronSpec::from_str("* * * * *").unwrap().compile().unwrap();
        assert!(!cron.verify(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)));
        assert!(cron.verify(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 0)));

        let cron = CronSpec::from_str("0 0 29 2 *").unwrap().compile().unwrap();
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 0, 0)));

        let cron = CronSpec::from_str("0 0 29 2 1").unwrap().compile().unwrap();
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 0, 0)));
        assert!(!cron.verify(NaiveDate::from_ymd(2020, 2, 29).and_hms(0, 0, 0)));

        let cron = CronSpec::from_str("*/2,*/3 0-10/3 * * *")
            .unwrap()
            .compile()
            .unwrap();
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 0, 0)));
        assert!(!cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 1, 0)));
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 2, 0)));
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 3, 0)));
        assert!(cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 4, 0)));
        assert!(!cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(0, 5, 0)));
        assert!(!cron.verify(NaiveDate::from_ymd(2016, 2, 29).and_hms(1, 0, 0)));
    }
}

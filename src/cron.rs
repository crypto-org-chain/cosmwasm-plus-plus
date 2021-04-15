use chrono::{Datelike, NaiveDateTime, Timelike};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitset::{BitSet, BitSetIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CronCompiled {
    pub minute: BitSet,
    pub hour: BitSet,
    pub mday: BitSet,
    pub month: BitSet,
    pub wday: BitSet,
}

impl CronCompiled {
    /// Verify the datetime matches cron spec
    pub fn verify(&self, datetime: NaiveDateTime) -> bool {
        println!("veirfy: {}", datetime);
        let time = datetime.time();
        let date = datetime.date();
        // SAFETY: guarantee of trait chrono::Timelike
        let minute = BitSetIndex::unsafe_new(time.minute() as u8);
        // SAFETY: guarantee of trait chrono::Timelike
        let hour = BitSetIndex::unsafe_new(time.hour() as u8);
        // SAFETY: guarantee of trait chrono::Datelike
        let mday = BitSetIndex::unsafe_new(date.day() as u8);
        // SAFETY: guarantee of trait chrono::Datelike
        let month = BitSetIndex::unsafe_new(date.month() as u8);
        // SAFETY: guarantee of num_days_from_sunday
        let wday = BitSetIndex::unsafe_new(date.weekday().num_days_from_sunday() as u8);
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
    }
}

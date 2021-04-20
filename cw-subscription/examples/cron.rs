use serde_json;
use std::env;

use cw_subscription::cron_spec::CronSpec;

fn main() {
    let args: Vec<_> = env::args().collect();
    let cron = args[1].parse::<CronSpec>().unwrap().compile().unwrap();
    println!("{}", serde_json::to_string(&cron).unwrap());
}

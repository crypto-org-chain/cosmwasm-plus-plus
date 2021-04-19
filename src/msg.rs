use chrono::offset::{FixedOffset, TimeZone};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Uint128};
use cw0::Expiration;

use crate::cron::CronCompiled;
use crate::error::ContractError;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Minimal native tokens deposit need for each plan, will refunded after deleted
    pub required_deposit_plan: Vec<Coin>,
    /// Minimal native tokens deposit need for each subscription, will refunded after deleted
    pub required_deposit_subscription: Vec<Coin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub params: Params,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ExecuteMsg {
    /// create plan, sender will be the plan owner
    CreatePlan(PlanContent),
    /// stop plan, sender must be the plan owner
    StopPlan { plan_id: Uint128 },
    /// sender subscribe to some plan
    /// If expiration is set, update if subscription exists
    Subscribe {
        plan_id: Uint128,
        expires: Expiration,
        next_collection_time: i64,
    },
    /// sender unsubscribe to some plan
    Unsubscribe { plan_id: Uint128 },
    /// Stop subscription on user's behalf, sender must be the plan owner
    UnsubscribeUser {
        plan_id: Uint128,
        subscriber: String,
    },
    /// Trigger collection of a batch of subscriptions
    Collection { items: Vec<CollectOne> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanContent {
    pub title: String,
    pub description: String,
    /// cw20 token address
    pub token: String,
    /// Amount to be collected for each period
    pub amount: Uint128,
    /// Crontab like specification for the plan
    pub cron: CronCompiled,
    /// timezone for the crontab logic
    pub tzoffset: i32,
}

impl PlanContent {
    pub fn validate(&self) -> Result<(), ContractError> {
        FixedOffset::east_opt(self.tzoffset).ok_or(ContractError::InvalidTimeZoneOffset)?;
        Ok(())
    }

    pub fn verify_timestamp(&self, ts: i64) -> bool {
        let datetime = FixedOffset::east(self.tzoffset)
            .timestamp(ts, 0)
            .naive_utc();
        self.cron.verify(datetime)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CollectOne {
    pub plan_id: Uint128,
    pub subscriber: String,
    pub current_collection_time: i64,
    pub next_collection_time: i64,
}

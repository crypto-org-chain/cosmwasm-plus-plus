use cosmwasm_std::{Addr, Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cron::CronCompiled;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Minimal native tokens need to deposit for each plan, will refunded after deleted
    pub minimal_deposit_plan: Coin,
    /// Minimal native tokens need to deposit for each subscription, will refunded after deleted
    pub minimal_deposit_subscription: Coin,
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
    Subscribe { plan_id: Uint128 },
    /// sender unsubscribe to some plan
    Unsubscribe { plan_id: Uint128 },
    /// Stop subscription on user's behalf, sender must be the plan owner
    UnsubscribeUser { subscription_id: Uint128 },
    /// Trigger collection of a batch of subscriptions
    TriggerCollection { items: Vec<CollectOne> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanContent {
    pub title: String,
    pub description: String,
    /// cw20 token address
    pub token: Addr,
    /// Amount to be collected for each collection
    pub amount: Uint128,
    /// Crontab like specification for the plan
    pub cron: CronCompiled,
    /// timezone for the crontab logic
    pub tzoffset: i32,
    /// The duration in seconds of subscription
    pub duration_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CollectOne {
    subscription_id: Uint128,
    current_collection_time: i64,
    next_collection_time: i64,
}

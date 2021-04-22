use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Plan, Subscription};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query one plan, response type is Plan<Addr>
    Plan { plan_id: Uint128 },
    /// List plans, support pagination, response type is PlansResponse
    ListPlans {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
    /// Query one subscription, response type is Subscription
    Subscription {
        plan_id: Uint128,
        subscriber: String,
    },
    /// List subscriptions, support pagination, response type is SubscriptionsResponse
    ListSubscriptions {
        plan_id: Uint128,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // TODO List all subscriptions of user
    // ListSubscriptionsOfUser {
    // },
    /// List collectible subscriptions, response type is SubscriptionsResponse
    CollectibleSubscriptions { limit: Option<u32> },
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<(Uint128, Addr, Subscription)>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
pub struct PlansResponse {
    pub plans: Vec<Plan>,
}

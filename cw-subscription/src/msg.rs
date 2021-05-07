use std::collections::HashSet;

use chrono::offset::{FixedOffset, TimeZone};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, Coin, Uint128};
use cw0::Expiration;

use crate::cron::CronCompiled;
use crate::error::ContractError;

const MAX_DESCRIPTION_LENGTH: usize = 5000;
const MAX_TITLE_LENGTH: usize = 140;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Minimal native tokens deposit need for each plan, will refunded after deleted
    pub required_deposit_plan: Vec<Coin>,
    /// Minimal native tokens deposit need for each subscription, will refunded after deleted
    pub required_deposit_subscription: Vec<Coin>,
}

fn has_duplicate_denom(items: &[Coin]) -> bool {
    let set = items.iter().map(|coin| &coin.denom).collect::<HashSet<_>>();
    set.len() != items.len()
}

impl Params {
    pub fn validate(&self) -> Result<(), ContractError> {
        if has_duplicate_denom(&self.required_deposit_plan) {
            return Err(ContractError::InvalidCoins);
        }
        if has_duplicate_denom(&self.required_deposit_subscription) {
            return Err(ContractError::InvalidCoins);
        }
        if self
            .required_deposit_plan
            .iter()
            .any(|coin| coin.amount == 0u128.into())
        {
            return Err(ContractError::InvalidCoins);
        }
        if self
            .required_deposit_subscription
            .iter()
            .any(|coin| coin.amount == 0u128.into())
        {
            return Err(ContractError::InvalidCoins);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub params: Params,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// create plan, sender will be the plan owner
    CreatePlan(PlanContent<String>),
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
    /// Update expires of subscription
    UpdateExpires {
        plan_id: Uint128,
        expires: Expiration,
    },
    /// Trigger collection of a batch of subscriptions
    Collect { items: Vec<CollectOne> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanContent<A> {
    pub title: String,
    pub description: String,
    /// cw20 token address
    pub token: A,
    /// Amount to be collected for each period
    pub amount: Uint128,
    /// Crontab like specification for the plan
    pub cron: CronCompiled,
    /// timezone for the crontab logic
    pub tzoffset: i32,
}

impl PlanContent<String> {
    pub fn validate(self, api: &dyn Api) -> Result<PlanContent<Addr>, ContractError> {
        if self.title.len() > MAX_TITLE_LENGTH {
            return Err(ContractError::TitleTooLong);
        }
        if self.description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(ContractError::DescriptionTooLong);
        }

        FixedOffset::east_opt(self.tzoffset).ok_or(ContractError::InvalidTimeZoneOffset)?;
        let token = api.addr_validate(&self.token)?;
        Ok(PlanContent::<Addr> {
            title: self.title,
            description: self.description,
            token,
            amount: self.amount,
            cron: self.cron,
            tzoffset: self.tzoffset,
        })
    }
}

impl<A> PlanContent<A> {
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

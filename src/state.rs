use cosmwasm_std::{Addr, Coin, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U128Key};
use serde::{Deserialize, Serialize};

use crate::msg::{Params, PlanContent};

/// Store the self-incremental unique ids for plans and subscriptions
pub const PLAN_ID: Item<Uint128> = Item::new("planid");
pub const SUBSCRIPTION_ID: Item<Uint128> = Item::new("subid");
/// Store contract params
pub const PARAMS: Item<Params> = Item::new("params");

/// Store the plans indexed by plan-id
/// plan-id -> Plan
pub const PLANS: Map<U128Key, Plan> = Map::new("plans");
/// Store the subscriptions indexed by subscription-id
/// subscription-id -> Subscription
pub const SUBSCRIPTIONS: Map<U128Key, Subscription> = Map::new("subs");

/// Subscriptions indexed by plan-id for enumeration
/// (plan-id, subscription-id) -> ()
pub const PLAN_SUBS: Map<(Uint128, Uint128), ()> = Map::new("plan-subs");
// /// Subscription queue ordered by expiration time
// /// (expiration-time, subscription-id) -> ()
// pub const Q_EXPIRATION: Map<(i64, Uint128), ()> = Map::new("subs-expiration");
/// Subscription queue ordered by next_collection_time
/// (next-collection-time, subscription-id) -> ()
pub const Q_COLLECTION: Map<(i64, Uint128), ()> = Map::new("subs-collection");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    id: Uint128,
    owner: Addr,
    content: PlanContent,
    deposit: Coin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    id: Uint128,
    plan_id: Uint128,
    subscriber: Addr,
    expiration_time: i64,
    last_collection_time: i64,
    next_collection_time: i64,
    deposit: Coin,
}

pub fn gen_plan_id(store: &mut dyn Storage) -> StdResult<Uint128> {
    let mut plan_id = PLAN_ID.may_load(store)?.unwrap_or(0u64.into());
    plan_id = plan_id.wrapping_add(1u64.into());
    // ensure id not used
    while store
        .get(&PLANS.key(U128Key::from(plan_id.u128())))
        .is_some()
    {
        plan_id = plan_id.wrapping_add(1u64.into());
    }
    PLAN_ID.save(store, &plan_id)?;
    Ok(plan_id)
}

pub fn gen_subscription_id(store: &mut dyn Storage) -> StdResult<Uint128> {
    let mut subscription_id = SUBSCRIPTION_ID.may_load(store)?.unwrap_or(0u64.into());
    subscription_id = subscription_id.wrapping_add(1u64.into());
    // ensure id not used
    while store
        .get(&SUBSCRIPTIONS.key(U128Key::from(subscription_id.u128())))
        .is_some()
    {
        subscription_id = subscription_id.wrapping_add(1u64.into());
    }
    PLAN_ID.save(store, &subscription_id)?;
    Ok(subscription_id)
}

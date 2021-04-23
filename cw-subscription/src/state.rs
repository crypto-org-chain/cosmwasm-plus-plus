use std::convert::TryInto;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Order, StdResult, Storage, Uint128};
use cw0::Expiration;
use cw_storage_plus::{Bound, I64Key, Item, Map, U128Key};

use crate::msg::{Params, PlanContent};

/// (plan-id, subscriber-address)
pub type SubscriptionKey<'a> = (U128Key, &'a str);

/// Store contract params
pub const PARAMS: Item<Params> = Item::new("params");

/// Store the self-incremental unique ids for plans
pub const PLAN_ID: Item<Uint128> = Item::new("planid");
/// Store the plans, `plan-id -> Plan`
pub const PLANS: Map<U128Key, Plan> = Map::new("plans");
/// Store the subscriptions, `(plan-id, subscriber) -> Subscription`
pub const SUBSCRIPTIONS: Map<SubscriptionKey, Subscription> = Map::new("plan-subs");

// /// Subscription queue ordered by expiration time
// /// (expiration-time, subscription-id) -> ()
// pub const Q_EXPIRATION: Map<(i64, Uint128), ()> = Map::new("subs-expiration");
/// Subscription queue ordered by next_collection_time
/// (next-collection-time, plan-id, subscriber) -> ()
pub const Q_COLLECTION: Map<(I64Key, SubscriptionKey), ()> = Map::new("q-collection");

const ZERO: Uint128 = Uint128::zero();

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    pub id: Uint128,
    pub owner: Addr,
    pub content: PlanContent<Addr>,
    pub deposit: Vec<Coin>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Subscription {
    pub expires: Expiration,
    /// Initialized to current block time created
    pub last_collection_time: i64,
    pub next_collection_time: i64,
    pub deposit: Vec<Coin>,
}

pub fn gen_plan_id(store: &mut dyn Storage) -> StdResult<Uint128> {
    let mut plan_id = PLAN_ID.may_load(store)?.unwrap_or(ZERO);
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

/// PANIC: if deserialization failed caused by corrupted storage
pub fn iter_subscriptions_by_plan(
    store: &dyn Storage,
    plan_id: Uint128,
    start_after: Option<Addr>,
) -> impl Iterator<Item = (Addr, Subscription)> + '_ {
    let start = start_after.map(|addr| Bound::exclusive(addr.as_ref()));
    SUBSCRIPTIONS
        .prefix(plan_id.u128().into())
        .range(store, start, None, Order::Ascending)
        .map(|mpair| {
            let (k, v) = mpair.unwrap();
            (Addr::unchecked(String::from_utf8(k).unwrap()), v)
        })
}

/// PANIC: if deserialization failed because of corrupted storage
pub fn iter_collectible_subscriptions(
    store: &dyn Storage,
    now: i64,
) -> impl Iterator<Item = (i64, Uint128, Addr)> + '_ {
    let minkey = Q_COLLECTION.key((I64Key::from(0), (U128Key::from(0), "")));
    let maxkey = Q_COLLECTION.key((
        I64Key::from(now.checked_add(1).unwrap()),
        (U128Key::from(0), ""),
    ));
    store
        .range(Some(&minkey), Some(&maxkey), Order::Ascending)
        .map(|(k, _)| {
            // decode key, TODO more elegant way?
            // skip the prefix
            let (_, k) = decode_key_step(&k).unwrap();

            let (s, k) = decode_key_step(k).unwrap();
            let collection_time = i64::from_be_bytes(s.try_into().unwrap());

            let (s, k) = decode_key_step(k).unwrap();
            let plan_id = u128::from_be_bytes(s.try_into().unwrap());

            // the last part is not prefixed with length
            let addr = Addr::unchecked(String::from_utf8(k.to_owned()).unwrap());
            (collection_time, plan_id.into(), addr)
        })
}

/// decode key, depends on the implemention details in cw-storage-plus
fn decode_key_step(buf: &[u8]) -> Option<(&[u8], &[u8])> {
    if buf.len() < 2 {
        return None;
    }
    let end = u16::from_be_bytes([buf[0], buf[1]]) as usize + 2;
    if buf.len() < end {
        return None;
    }
    Some((&buf[2..end], &buf[end..]))
}

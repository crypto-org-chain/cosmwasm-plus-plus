use cosmwasm_std::{Addr, Coin, Uint128};
use cw1155::Expiration;
use cw_storage_plus::{Item, Map};

/// Store the self-incremental unique ids for plans and subscriptions
pub const PLAN_ID: Item<Uint128> = Item::new("planid");
pub const SUBSCRIPTION_ID: Item<Uint128> = Item::new("subid");

/// Store the plans indexed by id
pub const PLANS: Map<Uint128, Plan> = Map::new("balances");
/// Store the approval status, `(owner, spender) -> expiration`
pub const APPROVES: Map<(&Addr, &Addr), Expiration> = Map::new("approves");
/// Store the tokens metadata url, also supports enumerating tokens,
/// An entry for token_id must exist as long as there's tokens in circulation.
pub const TOKENS: Map<&str, String> = Map::new("tokens");

pub struct Plan {
    plan_id: Uint128,
    owner: Addr,
    title: String,
    description: String,
    // FIXME do we need to support multiple coins here?
    price: Coin,
    duration_secs: u32,
    cron_spec: CronSpec,
    tzoffset: i32,
}

use std::convert::TryInto;

use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Storage, Uint128, WasmMsg,
};
use cw0::{Event, Expiration};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::{Bound, U128Key};

use crate::error::ContractError;
use crate::event::{
    CreatePlanEvent, StopPlanEvent, SubscribeEvent, UnsubscribeEvent, UpdateSubscriptionEvent,
};
use crate::msg::{CollectOne, ExecuteMsg, InitMsg, PlanContent};
use crate::query::{PlansResponse, QueryMsg, SubscriptionsResponse};
use crate::state::{
    gen_plan_id, iter_collectible_subscriptions, iter_subscriptions_by_plan, Plan, Subscription,
    PARAMS, PLANS, Q_COLLECTION, SUBSCRIPTIONS,
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    msg.params.validate()?;
    PARAMS.save(deps.storage, &msg.params)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePlan(content) => execute_create_plan(deps, info, content),
        ExecuteMsg::StopPlan { plan_id } => execute_stop_plan(deps, info, plan_id),
        ExecuteMsg::Subscribe {
            plan_id,
            expires,
            next_collection_time,
        } => execute_subscribe(deps, info, env, plan_id, expires, next_collection_time),
        ExecuteMsg::Unsubscribe { plan_id } => execute_unsubscribe(deps, info, plan_id),
        ExecuteMsg::UnsubscribeUser {
            plan_id,
            subscriber,
        } => execute_unsubscribe_user(deps, info, plan_id, subscriber),
        ExecuteMsg::UpdateExpires { plan_id, expires } => {
            execute_update_expires(deps, env, info, plan_id, expires)
        }
        ExecuteMsg::Collection { items } => execute_collection(deps, items),
    }
}

fn execute_create_plan(
    deps: DepsMut,
    info: MessageInfo,
    content: PlanContent<String>,
) -> Result<Response, ContractError> {
    let content = content.validate(deps.api)?;

    let params = PARAMS.load(deps.storage)?;
    let id = gen_plan_id(deps.storage)?;
    for required in params.required_deposit_plan.iter() {
        if !has_coins(&info.funds, required) {
            return Err(ContractError::NotEnoughDeposit);
        }
    }
    let plan = Plan {
        id,
        owner: info.sender,
        content,
        deposit: info.funds,
    };
    PLANS.save(deps.storage, id.u128().into(), &plan)?;

    let mut rsp = Response::default();
    CreatePlanEvent { plan_id: id }.add_attributes(&mut rsp);
    Ok(rsp)
}

fn execute_stop_plan(
    deps: DepsMut,
    info: MessageInfo,
    plan_id: Uint128,
) -> Result<Response, ContractError> {
    let plan = PLANS.load(deps.storage, plan_id.u128().into())?;
    if plan.owner != info.sender {
        return Err(ContractError::NotPlanOwner);
    }

    let mut rsp = Response::default();

    // Stop all subscriptions
    let subscriptions: Vec<_> = iter_subscriptions_by_plan(deps.storage, plan_id, None).collect();
    for (subscriber, sub) in subscriptions.into_iter() {
        UnsubscribeEvent {
            plan_id,
            subscriber: subscriber.as_str(),
        }
        .add_attributes(&mut rsp);

        let key = (plan_id.u128().into(), subscriber.as_str());
        SUBSCRIPTIONS.remove(deps.storage, key.clone());
        // delete in queue
        Q_COLLECTION.remove(deps.storage, (sub.next_collection_time.into(), key));
        rsp.messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: subscriber.into(),
            amount: sub.deposit,
        }));
    }

    // Delete plan
    PLANS.remove(deps.storage, plan_id.u128().into());
    rsp.messages.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: plan.owner.into(),
        amount: plan.deposit,
    }));
    StopPlanEvent { plan_id: plan.id }.add_attributes(&mut rsp);
    Ok(rsp)
}

fn execute_subscribe(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    plan_id: Uint128,
    expires: Expiration,
    next_collection_time: i64,
) -> Result<Response, ContractError> {
    // verify expires is valid
    if expires.is_expired(&env.block) {
        return Err(ContractError::InvalidExpires);
    }

    // verify subscription not exists
    let key = (plan_id.u128().into(), info.sender.as_str());
    let subkey = SUBSCRIPTIONS.key(key.clone());
    if deps.storage.get(&subkey).is_some() {
        return Err(ContractError::SubscriptionExists);
    }

    // verify deposit is enough
    let params = PARAMS.load(deps.storage)?;
    for required in params.required_deposit_subscription.iter() {
        if !has_coins(&info.funds, required) {
            return Err(ContractError::NotEnoughDeposit);
        }
    }

    // verify next_collection_time
    if next_collection_time <= env.block.time.try_into().unwrap() {
        return Err(ContractError::InvalidCollectionTime);
    }
    let plan = PLANS.load(deps.storage, plan_id.u128().into())?;
    if !plan.content.verify_timestamp(next_collection_time) {
        return Err(ContractError::InvalidCollectionTime);
    }

    // insert new subscription
    let sub = Subscription {
        expires,
        last_collection_time: env.block.time.try_into().unwrap(),
        next_collection_time,
        deposit: info.funds,
    };
    subkey.save(deps.storage, &sub)?;
    Q_COLLECTION.save(deps.storage, (next_collection_time.into(), key), &())?;

    let mut rsp = Response::default();
    SubscribeEvent {
        plan_id,
        subscriber: info.sender.as_str(),
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

fn unsubscribe(
    storage: &mut dyn Storage,
    plan_id: Uint128,
    subscriber: Addr,
) -> StdResult<CosmosMsg> {
    // delete subscription
    let key = (U128Key::from(plan_id.u128()), subscriber.as_str());
    let sub = SUBSCRIPTIONS.load(storage, key.clone())?;
    SUBSCRIPTIONS.remove(storage, key.clone());
    // delete in queue
    Q_COLLECTION.remove(storage, (sub.next_collection_time.into(), key));
    Ok(CosmosMsg::Bank(BankMsg::Send {
        to_address: subscriber.into(),
        amount: sub.deposit,
    }))
}

fn execute_unsubscribe(
    deps: DepsMut,
    info: MessageInfo,
    plan_id: Uint128,
) -> Result<Response, ContractError> {
    let mut rsp = Response::default();
    UnsubscribeEvent {
        plan_id,
        subscriber: info.sender.as_str(),
    }
    .add_attributes(&mut rsp);

    let refund_msg = unsubscribe(deps.storage, plan_id, info.sender)?;
    rsp.messages.push(refund_msg);
    Ok(rsp)
}

fn execute_unsubscribe_user(
    deps: DepsMut,
    info: MessageInfo,
    plan_id: Uint128,
    subscriber: String,
) -> Result<Response, ContractError> {
    let subscriber = deps.api.addr_validate(&subscriber)?;
    // load and plan, verify info.sender is plan owner
    let plan = PLANS.load(deps.storage, plan_id.u128().into())?;
    if plan.owner != info.sender {
        return Err(ContractError::NotPlanOwner);
    }

    let mut rsp = Response::default();
    UnsubscribeEvent {
        plan_id,
        subscriber: subscriber.as_str(),
    }
    .add_attributes(&mut rsp);

    let refund_msg = unsubscribe(deps.storage, plan_id, subscriber)?;
    rsp.messages.push(refund_msg);
    Ok(rsp)
}

fn execute_update_expires(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    plan_id: Uint128,
    expires: Expiration,
) -> Result<Response, ContractError> {
    if expires.is_expired(&env.block) {
        return Err(ContractError::InvalidExpires);
    }
    let key = SUBSCRIPTIONS.key((plan_id.u128().into(), info.sender.as_str()));
    let mut subscription = key.load(deps.storage)?;
    subscription.expires = expires;
    key.save(deps.storage, &subscription)?;

    let mut rsp = Response::default();
    UpdateSubscriptionEvent {
        plan_id,
        subscriber: info.sender.as_str(),
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

fn execute_collection(deps: DepsMut, items: Vec<CollectOne>) -> Result<Response, ContractError> {
    let mut rsp = Response::default();
    for item in items.iter() {
        if item.next_collection_time <= item.current_collection_time {
            // TODO handle failure
            continue;
        }
        let subscriber = deps.api.addr_validate(&item.subscriber)?;

        // load plan and subscription
        let plan = PLANS.load(deps.storage, item.plan_id.u128().into())?;
        let key = (item.plan_id.u128().into(), subscriber.as_str());
        let mut subscription = SUBSCRIPTIONS.load(deps.storage, key.clone())?;
        if item.current_collection_time <= subscription.last_collection_time {
            // TODO handle failure
            continue;
        }
        // verify collection time match cron spec
        if !(plan.content.verify_timestamp(item.current_collection_time)
            && plan.content.verify_timestamp(item.next_collection_time))
        {
            // TODO handle failure
            continue;
        }

        // do cw20 transfer
        // TODO handle transfer failure with submessage callback
        rsp.messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: plan.content.token.into(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: subscriber.clone().into(),
                recipient: plan.owner.into(),
                amount: plan.content.amount,
            })?,
            send: vec![],
        }));

        // update next_collection_time
        subscription.last_collection_time = item.current_collection_time;
        Q_COLLECTION.remove(
            deps.storage,
            (subscription.next_collection_time.into(), key.clone()),
        );
        subscription.next_collection_time = item.next_collection_time;
        Q_COLLECTION.save(
            deps.storage,
            (subscription.next_collection_time.into(), key.clone()),
            &(),
        )?;
        SUBSCRIPTIONS.save(deps.storage, key, &subscription)?;
    }

    Ok(rsp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Plan { plan_id } => to_binary(&PLANS.load(deps.storage, plan_id.u128().into())?),
        QueryMsg::Subscription {
            plan_id,
            subscriber,
        } => to_binary(&SUBSCRIPTIONS.load(deps.storage, (plan_id.u128().into(), &subscriber))?),
        QueryMsg::ListSubscriptions {
            plan_id,
            start_after,
            limit,
        } => query_subscriptions(deps, plan_id, start_after, limit),
        QueryMsg::ListPlans { start_after, limit } => query_plans(deps, start_after, limit),
        QueryMsg::CollectibleSubscriptions { limit } => {
            query_collectible_subscriptions(deps, env, limit)
        }
    }
}

fn query_plans(deps: Deps, start_after: Option<Uint128>, limit: Option<u32>) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|id| Bound::Exclusive(U128Key::from(id.u128()).into()));
    let plans = PLANS
        .range(deps.storage, start_after, None, Order::Ascending)
        .map(|mpair| mpair.unwrap().1)
        .take(limit)
        .collect();
    to_binary(&PlansResponse { plans })
}

fn query_subscriptions(
    deps: Deps,
    plan_id: Uint128,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|addr| deps.api.addr_validate(&addr).unwrap());
    let subscriptions = iter_subscriptions_by_plan(deps.storage, plan_id, start_after)
        .map(|(subscriber, sub)| (plan_id, subscriber, sub))
        .take(limit)
        .collect();
    to_binary(&SubscriptionsResponse { subscriptions })
}

fn query_collectible_subscriptions(deps: Deps, env: Env, limit: Option<u32>) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let items: Vec<_> =
        iter_collectible_subscriptions(deps.storage, env.block.time.try_into().unwrap())
            .take(limit)
            .collect();
    let subscriptions = items
        .into_iter()
        .map(|(_, plan_id, subscriber)| {
            SUBSCRIPTIONS
                .load(deps.storage, (plan_id.u128().into(), subscriber.as_str()))
                .map(|sub| (plan_id, subscriber, sub))
        })
        .collect::<StdResult<Vec<_>>>()?;
    to_binary(&SubscriptionsResponse { subscriptions })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr};

    use super::*;

    use crate::cron_spec::CronSpec;
    use crate::msg::Params;

    #[test]
    fn check_basic_flow() {
        // instantiate a contract
        // create plan
        // subscribe
        // collect payment
        // query
        let _native_token = "cro".to_owned();
        let merchant = String::from("merchant");
        let user = String::from("user");
        let token_contract = String::from("cw20-contract");

        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {
            params: Params {
                required_deposit_plan: vec![],
                required_deposit_subscription: vec![],
            },
        };
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let content = PlanContent::<String> {
            title: "test plan1".to_owned(),
            description: "test plan1".to_owned(),
            token: token_contract.clone(),
            amount: 1u128.into(),
            cron: "* * * * *".parse::<CronSpec>().unwrap().compile().unwrap(),
            tzoffset: 0,
        };
        let plan_msg = ExecuteMsg::CreatePlan(content.clone());
        let rsp = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(merchant.as_ref(), &[]),
            plan_msg,
        )
        .unwrap();
        let plan_id: Uint128 = rsp.attributes[1].value.parse::<u128>().unwrap().into();

        // query one plan
        let plan: Plan =
            from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Plan { plan_id }).unwrap())
                .unwrap();
        assert_eq!(
            plan,
            Plan {
                id: 1u128.into(),
                owner: Addr::unchecked(merchant),
                content: content.validate(&deps.api).unwrap(),
                deposit: vec![]
            }
        );

        // list plans
        let plans: PlansResponse = from_binary(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::ListPlans {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(plans.plans.len(), 1);
        let plans: PlansResponse = from_binary(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::ListPlans {
                    start_after: Some(plans.plans[0].id),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(plans.plans.len(), 0);

        // subscribe failures
        assert_matches!(
            execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user.as_ref(), &[]),
                ExecuteMsg::Subscribe {
                    plan_id: 1u128.into(),
                    expires: Expiration::Never {},
                    next_collection_time: 0,
                },
            ),
            Err(ContractError::InvalidCollectionTime)
        );
        assert_matches!(
            execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user.as_ref(), &[]),
                ExecuteMsg::Subscribe {
                    plan_id: 1u128.into(),
                    expires: Expiration::Never {},
                    next_collection_time: 1_571_797_420,
                },
            ),
            Err(ContractError::InvalidCollectionTime)
        );
        // subscribe succeed
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(user.as_ref(), &[]),
            ExecuteMsg::Subscribe {
                plan_id: 1u128.into(),
                expires: Expiration::Never {},
                next_collection_time: 1_571_797_440,
            },
        )
        .unwrap();

        // query collectible subscriptions
        {
            let mut env = env.clone();
            let rsp: SubscriptionsResponse = from_binary(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::CollectibleSubscriptions { limit: None },
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(rsp.subscriptions.len(), 0);

            env.block.time = 1_571_797_440;
            let rsp: SubscriptionsResponse = from_binary(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::CollectibleSubscriptions { limit: None },
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(rsp.subscriptions.len(), 1);

            // collect payment
            let (plan_id, subscriber, sub) = rsp.subscriptions[0].clone();

            // test validations
            assert_eq!(
                execute(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(user.as_ref(), &[]),
                    ExecuteMsg::Collection {
                        items: vec![CollectOne {
                            plan_id,
                            subscriber: subscriber.clone().into(),
                            current_collection_time: 0,
                            next_collection_time: sub.next_collection_time + 60,
                        }],
                    },
                )
                .unwrap()
                .messages
                .len(),
                0
            );
            assert_eq!(
                execute(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(user.as_ref(), &[]),
                    ExecuteMsg::Collection {
                        items: vec![CollectOne {
                            plan_id,
                            subscriber: subscriber.clone().into(),
                            current_collection_time: sub.next_collection_time,
                            next_collection_time: sub.next_collection_time + 61,
                        }],
                    },
                )
                .unwrap()
                .messages
                .len(),
                0
            );

            // success path
            let rsp = execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user.as_ref(), &[]),
                ExecuteMsg::Collection {
                    items: vec![CollectOne {
                        plan_id,
                        subscriber: subscriber.into(),
                        current_collection_time: sub.next_collection_time,
                        next_collection_time: sub.next_collection_time + 60,
                    }],
                },
            )
            .unwrap();
            // one cw20 transfer message for each successful payment collection
            assert_eq!(rsp.messages.len(), 1);
        }
    }
}

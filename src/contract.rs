use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Storage, Uint128, WasmMsg,
};
use cw0::{Event, Expiration};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::U128Key;

use crate::error::ContractError;
use crate::event::{
    CreatePlanEvent, StopPlanEvent, SubscribeEvent, UnsubscribeEvent, UpdateSubscriptionEvent,
};
use crate::msg::{CollectOne, ExecuteMsg, InitMsg, PlanContent};
use crate::query::QueryMsg;
use crate::state::{
    gen_plan_id, iter_subscriptions_by_plan, Plan, Subscription, PARAMS, PLANS, Q_COLLECTION,
    SUBSCRIPTIONS,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<Response> {
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
    content: PlanContent,
) -> Result<Response, ContractError> {
    content.validate()?;

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
    let subscriptions: Vec<_> = iter_subscriptions_by_plan(deps.storage, plan_id).collect();
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
    let plan = PLANS.load(deps.storage, plan_id.u128().into())?;
    plan.content.verify_timestamp(next_collection_time);

    // insert new subscription
    let sub = Subscription {
        expires,
        last_collection_time: None,
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
        if let Some(last_collection_time) = subscription.last_collection_time {
            if item.current_collection_time <= last_collection_time {
                // TODO handle failure
                continue;
            }
        }
        // verify collection time match cron spec
        if !plan.content.verify_timestamp(item.current_collection_time)
            || !plan.content.verify_timestamp(item.next_collection_time)
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
        subscription.last_collection_time = Some(item.current_collection_time);
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
    Ok(Binary::default())
}

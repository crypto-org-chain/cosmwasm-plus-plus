from datetime import datetime, timezone
from pathlib import Path

from .test_wrap_contract import WRAP_TOKEN


def round_up_minute(t):
    return t + (60 - t % 60)


def round_down_minute(t):
    return t - t % 60


def test_subscription(cluster):
    creator = cluster.address("community")
    user = cluster.address("ecosystem")
    print("deploy cw20 and subscription contracts")
    cw20_contract = cluster.construct(
        Path(__file__).parent / "artifacts/cw20_bonding.wasm",
        WRAP_TOKEN,
        creator,
        label="wcosm",
    )
    contract = cluster.construct(
        Path(__file__).parent
        / "../target/wasm32-unknown-unknown/release/cw_subscription.wasm",
        {
            "params": {
                "required_deposit_plan": [{"amount": "1000", "denom": "ucosm"}],
                "required_deposit_subscription": [{"amount": "1000", "denom": "ucosm"}],
            },
        },
        creator,
        label="subscription",
    )
    # ./target/debug/examples/cron "* * * * *"
    cron = {
        "minute": 1152921504606846975,
        "hour": 16777215,
        "mday": 4294967294,
        "month": 8190,
        "wday": 127,
    }
    print("create per-minute subscription plan")
    events = cluster.execute(
        contract,
        {
            "create_plan": {
                "title": "test plan",
                "description": "test plan",
                "token": cw20_contract,
                "amount": "1000000",
                "cron": cron,
                "tzoffset": 0,
            },
        },
        creator,
        amount=1000,
    )
    plan_id = int(events[2]["plan_id"])

    print("subscribe")
    next_collection_time = round_up_minute(int(datetime.now().timestamp()))
    events = cluster.execute(
        contract,
        {
            "subscribe": {
                "plan_id": str(plan_id),
                "expires": {"never": {}},
                "next_collection_time": next_collection_time,
            },
        },
        user,
        amount=1000,
    )
    assert events[2] == {
        "contract_address": contract,
        "action": "subscribe",
        "plan_id": str(plan_id),
        "subscriber": user,
    }

    print("wait for collection time")
    cluster.wait_for_block_time(
        datetime.utcfromtimestamp(next_collection_time).replace(tzinfo=timezone.utc)
    )
    rsp = cluster.query(contract, {"collectible_subscriptions": {}})
    # the only subscription should be collectible
    print("subscriptions", rsp)
    assert len(rsp["subscriptions"]) == 1

    print("deposit cw20 tokens and set approval")
    _ = cluster.execute(
        cw20_contract,
        {
            "buy": {},
        },
        user,
        amount=1000000,
    )
    cluster.execute(
        cw20_contract,
        {
            "increase_allowance": {
                "spender": contract,
                "amount": "1000000",
            },
        },
        user,
    )

    print("collect payments")
    events = cluster.execute(
        contract,
        {
            "collect": {
                "items": [
                    {
                        "plan_id": plan_id,
                        "subscriber": subscriber,
                        "current_collection_time": next_collection_time,
                        "next_collection_time": round_up_minute(next_collection_time),
                    }
                    for (plan_id, subscriber, _) in rsp["subscriptions"]
                ]
            },
        },
        user,  # anyone can do
    )
    assert events[1] == {
        "contract_address": cw20_contract,
        "action": "transfer_from",
        "from": user,
        "to": creator,
        "by": contract,
        "amount": "1000000",
    }, "invalid events"

    print("check cw20 token balances")
    rsp = cluster.query(
        cw20_contract,
        {
            "balance": {
                "address": creator,
            },
        },
    )
    assert rsp == {"balance": "1000000"}
    rsp = cluster.query(
        cw20_contract,
        {
            "balance": {
                "address": user,
            },
        },
    )
    assert rsp == {"balance": "0"}

    print("unsubscribe")
    old_balance = cluster.balances(user)["ucosm"]
    events = cluster.execute(
        contract,
        {
            "unsubscribe": {"plan_id": str(plan_id)},
        },
        user,
    )
    assert events[2] == {
        "contract_address": contract,
        "action": "unsubscribe",
        "plan_id": str(plan_id),
        "subscriber": user,
    }
    print("check native token refunded")
    assert cluster.balances(user)["ucosm"] == old_balance + 1000

    print("stop plan")
    old_balance = cluster.balances(creator)["ucosm"]
    events = cluster.execute(
        contract,
        {
            "stop_plan": {"plan_id": str(plan_id)},
        },
        creator,
    )
    assert events[2] == {
        "contract_address": contract,
        "action": "stop-plan",
        "plan_id": str(plan_id),
    }
    print("check native token refunded")
    assert cluster.balances(creator)["ucosm"] == old_balance + 1000

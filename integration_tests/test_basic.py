import json


def parse_events(rsp):
    return [
        {attr["key"]: attr["value"] for attr in evt["attributes"]}
        for evt in json.loads(rsp["raw_log"])[0]["events"]
    ]


def test_basic(cluster):
    addr = cluster("keys", "show", "community", "-a").strip().decode()
    print("addr", addr)
    rsp = json.loads(
        cluster(
            "tx",
            "wasm",
            "store",
            "target/wasm32-unknown-unknown/release/cw_subscription.wasm",
            "-y",
            from_=addr,
            gas=2000000,
        )
    )
    assert rsp.get("code", 0) == 0, rsp["raw_log"]
    code_id = int(parse_events(rsp)[0]["code_id"])
    init_msg = {
        "params": {
            "required_deposit_plan": [{"amount": "1000", "denom": "ucosm"}],
            "required_deposit_subscription": [{"amount": "1000", "denom": "ucosm"}],
        },
    }
    rsp = json.loads(
        cluster(
            "tx",
            "wasm",
            "instantiate",
            code_id,
            json.dumps(init_msg),
            "-y",
            label="subscription",
            admin=addr,
            from_=addr,
        )
    )
    assert rsp.get("code", 0) == 0, rsp["raw_log"]
    _ = parse_events(rsp)[0]["contract_address"]

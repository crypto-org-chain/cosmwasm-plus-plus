from pathlib import Path

WRAP_TOKEN = {
    "name": "wrapped cosm",
    "symbol": "wcosm",
    "decimals": 6,
    "reserve_denom": "ucosm",
    "reserve_decimals": 6,
    "curve_type": {
        "constant": {"value": "1", "scale": 0},
    },
}


def test_wrap_contract(cluster):
    """
    test cw20 contract that wrap native token
    use the cw20-bonding example contract in cosmwasm-plus repo.
    """
    creator = cluster.address("community")
    contract = cluster.construct(
        Path(__file__).parent / "artifacts/cw20_bonding.wasm",
        WRAP_TOKEN,
        creator,
        label="wcosm",
    )
    user = cluster.address("ecosystem")
    orig_balance = cluster.balances(user)["ucosm"]
    _ = cluster.execute(
        contract,
        {
            "buy": {},
        },
        user,
        amount=1000000,
    )

    rsp = cluster.query(
        contract,
        {
            "balance": {
                "address": user,
            },
        },
    )
    # native decreased and cw20 increased
    assert rsp == {"balance": "1000000"}
    assert cluster.balances(user)["ucosm"] == orig_balance - 1000000

    _ = cluster.execute(
        contract,
        {
            "burn": {"amount": "1000000"},
        },
        user,
    )
    rsp = cluster.query(
        contract,
        {
            "balance": {
                "address": user,
            },
        },
    )

    # cw20 decreased and native tokens recovered
    assert rsp == {"balance": "0"}
    assert cluster.balances(user)["ucosm"] == orig_balance

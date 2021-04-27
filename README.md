- [cw-subscription](cw-subscription): Subscription module implemented as CosmWasm smart contract.

### Prerequisite

- recent version of nightly rust

### Integration tests

#### Prerequisite

- [Nix](https://nixos.org/download.html)

#### Run

```
# Build contracts
RUSTFLAGS="-C link-arg=-s" cargo build --release --target=wasm32-unknown-unknown --locked
# Run tests
nix-shell --run "pytest integration_tests"
```
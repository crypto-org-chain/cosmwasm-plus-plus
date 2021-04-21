use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("not enough deposit")]
    NotEnoughDeposit,
    #[error("wrong deposit coin")]
    WrongDepositCoin,
    #[error("plan not exists")]
    PlanNotExists,
    #[error("invalid expires")]
    InvalidExpires,
    #[error("subscription expired")]
    SubscriptionExpired,
    #[error("subscription exists")]
    SubscriptionExists,
    #[error("invalid timezone offset")]
    InvalidTimeZoneOffset,
    #[error("the sender is not plan owner")]
    NotPlanOwner,
    #[error("invalid input coins")]
    InvalidCoins,
}

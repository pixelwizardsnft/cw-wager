use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Method Not Implemented")]
    NotImplemented {},

    #[error("Token Already Wagered")]
    AlreadyWagered {},

    #[error("Wager Still Active")]
    WagerActive {},

    #[error("Token Not Matchmaking")]
    NotMatchmaking {},

    #[error("Invalid Parameter: {param:?}")]
    InvalidParameter { param: String },

    #[error("Unique Error: {val:?}")]
    CustomErrorParam { val: String },
}

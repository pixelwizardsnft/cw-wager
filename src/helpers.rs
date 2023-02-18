use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Coin, StdResult, SubMsg};
use sg_std::StargazeMsgWrapper;

/// CwWagerContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[cw_serde]
pub struct CwWagerContract(pub Addr);

impl CwWagerContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }
}

// Send native tokens to another address
pub fn send_tokens(to: Addr, balance: Coin) -> StdResult<SubMsg<StargazeMsgWrapper>> {
    let msg = BankMsg::Send {
        to_address: to.into_string(),
        amount: vec![balance],
    };

    let exec = SubMsg::<StargazeMsgWrapper>::new(msg);

    Ok(exec)
}

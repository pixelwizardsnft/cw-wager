use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::{
    config::ParamInfo,
    state::{Config, Currency, Token, TokenStatus, WagerExport},
};

#[cw_serde]
pub struct InstantiateMsg {
    pub max_currencies: u8,
    pub amounts: Vec<Uint128>,
    pub expiries: Vec<u64>,
    pub fee_bps: u64,
    pub fairburn_bps: u64,
    pub fee_address: String,
    pub collection_address: String,
    pub matchmaking_expiry: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Privileged
    UpdateConfig {
        params: ParamInfo,
    },

    /// Use Authz
    SetWinner {
        wager_key: (Token, Token),
        prev_prices: (u64, u64),
        current_prices: (u64, u64),
    },

    /// User-facing
    Wager {
        token: Token,
        currency: Currency,
        against_currencies: Vec<Currency>,
        expiry: u64,
    },
    Cancel {
        token: Token,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(WagersResponse)]
    Wagers {},
    #[returns(WagerResponse)]
    Wager { token: Token },
    #[returns(TokenStatusResponse)]
    TokenStatus { token: Token },
    #[returns(ConfigResponse)]
    Config {},
}

// We define a custom struct for each query response

#[cw_serde]
pub struct WagersResponse {
    pub wagers: Vec<WagerExport>,
}

#[cw_serde]
pub struct WagerResponse {
    pub wager: WagerExport,
}

#[cw_serde]
pub struct TokenStatusResponse {
    pub token_status: TokenStatus,
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

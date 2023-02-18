use crate::{error::ContractError, state::CONFIG};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, DepsMut, Response, Uint128};
use sg_std::StargazeMsgWrapper;

#[cw_serde]
pub struct ParamInfo {
    pub max_currencies: Option<u8>,
    pub amounts: Option<Vec<Uint128>>,
    pub expiries: Option<Vec<u64>>,
    pub fee_bps: Option<u64>,
    pub fairburn_bps: Option<u64>,
    pub fee_address: Option<String>,
    pub collection_address: Option<String>,
    pub matchmaking_expiry: Option<u64>,
}

pub fn execute_update_params(
    deps: DepsMut,
    param_info: ParamInfo,
) -> Result<Response<StargazeMsgWrapper>, ContractError> {
    let ParamInfo {
        max_currencies,
        amounts,
        expiries,
        fee_bps,
        fee_address,
        collection_address,
        matchmaking_expiry,
        fairburn_bps,
    } = param_info;

    let mut params = CONFIG.load(deps.storage)?;

    if let Some(max_currencies) = max_currencies {
        if max_currencies < 1 {
            return Err(ContractError::InvalidParameter {
                param: "max_currencies".into(),
            });
        }

        params.max_currencies = max_currencies;
    }

    if let Some(amounts) = amounts {
        params.amounts = amounts;
    }

    if let Some(expiries) = expiries {
        params.expiries = expiries;
    }

    params.fee_percent = fee_bps.map(Decimal::percent).unwrap_or(params.fee_percent);

    params.fairburn_percent = fairburn_bps
        .map(Decimal::percent)
        .unwrap_or(params.fairburn_percent);

    if let Some(fee_address) = fee_address {
        params.fee_address = deps.api.addr_validate(&fee_address)?;
    }

    if let Some(collection_address) = collection_address {
        params.collection_address = deps.api.addr_validate(&collection_address)?;
    }

    if let Some(matchmaking_expiry) = matchmaking_expiry {
        params.matchmaking_expiry = matchmaking_expiry;
    }

    CONFIG.save(deps.storage, &params)?;

    Ok(Response::new().add_attribute("action", "update_params"))
}

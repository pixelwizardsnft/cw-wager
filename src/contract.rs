#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Order, StdError,
    StdResult,
};
use cw2::set_contract_version;
use semver::Version;
use sg_std::Response;

use crate::config::execute_update_params;
use crate::error::ContractError;
use crate::execute::{execute_cancel, execute_set_winner, execute_wager};
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MatchmakingResponse, QueryMsg, TokenStatusResponse,
    WagerResponse, WagersResponse,
};
use crate::state::{
    wagers, Config, MatchmakingItem, MatchmakingItemExport, Token, TokenStatus, Wager, WagerExport,
    WagerInfo, CONFIG, MATCHMAKING, NFT,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-wager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let InstantiateMsg {
        max_currencies,
        amounts,
        expiries,
        fee_bps,
        fee_address,
        collection_address,
        matchmaking_expiry,
        fairburn_bps,
    } = msg;

    if max_currencies < 1 {
        return Err(ContractError::InvalidParameter {
            param: "max_currencies".into(),
        });
    }

    if amounts.is_empty() {
        return Err(ContractError::InvalidParameter {
            param: "amounts".into(),
        });
    }

    if expiries.is_empty() {
        return Err(ContractError::InvalidParameter {
            param: "expiries".into(),
        });
    }

    if matchmaking_expiry < 60 {
        return Err(ContractError::InvalidParameter {
            param: "matchmaking_expiry".into(),
        });
    }

    let fee_address = deps.api.addr_validate(&fee_address)?;
    let collection_address = deps.api.addr_validate(&collection_address)?;

    CONFIG.save(
        deps.storage,
        &Config {
            max_currencies,
            amounts,
            expiries,
            fee_percent: Decimal::percent(fee_bps),
            fairburn_percent: Decimal::percent(fairburn_bps),
            fee_address,
            collection_address,
            matchmaking_expiry,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("sender", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { params } => {
            admin_only(deps.as_ref(), info)?;
            execute_update_params(deps, params)
        }
        ExecuteMsg::SetWinner {
            wager_key,
            prev_prices,
            current_prices,
        } => {
            admin_only(deps.as_ref(), info)?;
            execute_set_winner(deps, env, wager_key, prev_prices, current_prices)
        }
        ExecuteMsg::Wager {
            token,
            currency,
            against_currencies,
            expiry,
        } => execute_wager(deps, env, info, token, currency, against_currencies, expiry),
        ExecuteMsg::Cancel { token } => execute_cancel(deps, info, token),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }

    // use semver
    let version = Version::parse(&ver.version).unwrap();
    let contract_version = Version::parse(CONTRACT_VERSION).unwrap();

    if version.ge(&contract_version) {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // do any desired state migrations...

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Wagers {} => to_binary(&query_wagers(deps)?),
        QueryMsg::Wager { token } => to_binary(&query_wager(deps, token)?),
        QueryMsg::Matchmaking {} => to_binary(&query_matchmaking(deps)?),
        QueryMsg::TokenStatus { token } => to_binary(&query_token_status(deps, token)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_wagers(deps: Deps) -> StdResult<WagersResponse> {
    let config = CONFIG.load(deps.storage)?;

    let wagers = wagers()
        .idx
        .id
        .range(deps.storage, None, None, Order::Ascending)
        .map(|v| export_wager(v.unwrap().1, config.collection_address.clone()))
        .collect::<Vec<_>>();

    Ok(WagersResponse { wagers })
}

pub fn query_wager(deps: Deps, token: Token) -> StdResult<WagerResponse> {
    let config = CONFIG.load(deps.storage)?;

    // Find the wager with the key containing the token and return it as WagerExport
    let wager = wagers()
        .idx
        .id
        .range(deps.storage, None, None, Order::Ascending)
        .find(|item| {
            item.as_ref()
                .map(|(_, w)| w.id.0 == token || w.id.1 == token)
                .unwrap_or(false)
        })
        .map(|v| export_wager(v.unwrap().1, config.collection_address));

    match wager {
        Some(wager) => Ok(WagerResponse { wager }),
        None => Err(cosmwasm_std::StdError::NotFound {
            kind: "wager".into(),
        }),
    }
}

pub fn query_matchmaking(deps: Deps) -> StdResult<MatchmakingResponse> {
    let config = CONFIG.load(deps.storage)?;

    let matchmaking = MATCHMAKING
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|v| export_matchmaking(v.clone().0, v.clone().1, config.collection_address.clone()))
        .collect();

    Ok(MatchmakingResponse { matchmaking })
}

pub fn query_token_status(deps: Deps, token: Token) -> StdResult<TokenStatusResponse> {
    // If there is a Wager for the token, return TokenStatus::Wager(Wager).
    // If there is a MatchmakingItem for the token, return TokenStatus::Matchmaking(MatchmakingItem).
    // If there is no Wager or MatchmakingItem for the token, return TokenStatus::None.

    let config = CONFIG.load(deps.storage)?;

    let wager = wagers()
        .idx
        .id
        .range(deps.storage, None, None, Order::Ascending)
        .find(|item| {
            item.as_ref()
                .map(|(_, w)| w.id.0 == token || w.id.1 == token)
                .unwrap_or(false)
        })
        .map(|v| export_wager(v.unwrap().1, config.collection_address.clone()));

    if let Some(wager) = wager {
        return Ok(TokenStatusResponse {
            token_status: TokenStatus::Wager(wager),
        });
    }

    let matchmaking_item =
        MATCHMAKING
            .may_load(deps.storage, token)?
            .ok_or(cosmwasm_std::StdError::NotFound {
                kind: "matchmaking_item".into(),
            });

    if matchmaking_item.is_ok() {
        let MatchmakingItem {
            expires_at,
            currency,
            against_currencies,
            expiry,
            amount,
        } = matchmaking_item?;
        Ok(TokenStatusResponse {
            token_status: TokenStatus::Matchmaking(MatchmakingItemExport {
                token: NFT {
                    collection: config.collection_address,
                    token_id: token,
                },
                expires_at,
                currency,
                against_currencies,
                expiry,
                amount,
            }),
        })
    } else {
        Ok(TokenStatusResponse {
            token_status: TokenStatus::None,
        })
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn export_wager(v: Wager, collection: Addr) -> WagerExport {
    WagerExport {
        amount: v.amount,
        expires_at: v.expires_at,
        wagers: (
            WagerInfo {
                token: NFT {
                    collection: collection.clone(),
                    token_id: v.id.0,
                },
                currency: v.currencies.0,
            },
            WagerInfo {
                token: NFT {
                    collection,
                    token_id: v.id.1,
                },
                currency: v.currencies.1,
            },
        ),
    }
}

fn export_matchmaking(
    token_id: u64,
    v: MatchmakingItem,
    collection: Addr,
) -> MatchmakingItemExport {
    MatchmakingItemExport {
        token: NFT {
            collection,
            token_id,
        },
        currency: v.currency,
        against_currencies: v.against_currencies,
        expires_at: v.expires_at,
        expiry: v.expiry,
        amount: v.amount,
    }
}

fn admin_only(deps: Deps, info: MessageInfo) -> Result<Empty, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.fee_address {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(Empty {})
    }
}

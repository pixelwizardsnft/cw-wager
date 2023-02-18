use cosmwasm_std::{coin, Order, Uint128};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo};
use cw721_base::helpers::Cw721Contract;
use cw_utils::must_pay;
use sg1::fair_burn;
use sg_std::{Response, NATIVE_DENOM};

use crate::contract::query_token_status;
use crate::error::ContractError;
use crate::helpers::send_tokens;
use crate::state::{
    Currency, MatchmakingItem, Token, TokenStatus, Wager, CONFIG, MATCHMAKING, WAGERS,
};

#[allow(clippy::too_many_arguments)]
pub fn execute_wager(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: Token,
    currency: Currency,
    against_currencies: Vec<Currency>,
    expiry: u64,
) -> Result<Response, ContractError> {
    let (collection, token_id) = token.clone();

    let amount = must_pay(&info, NATIVE_DENOM)?;

    // Verify that the token's collection address is the same as the contract address in config
    let config = CONFIG.load(deps.storage)?;
    if collection != config.collection_address {
        return Err(ContractError::Unauthorized {});
    }

    // Verify that the currencies do not exceed the maximum amount
    if against_currencies.len() > config.max_currencies as usize {
        return Err(ContractError::InvalidParameter {
            param: "against_currencies".into(),
        });
    };

    // Verify that the currencies do not include the base currency
    if against_currencies.contains(&currency) {
        return Err(ContractError::InvalidParameter {
            param: "against_currencies".into(),
        });
    };

    // Verify that the expiry is within the list of allowed expiries
    if !config.expiries.contains(&expiry) {
        return Err(ContractError::InvalidParameter {
            param: "expiry".into(),
        });
    };

    // Verify that the amount is within the list of allowed amounts
    if !config.amounts.contains(&amount) {
        return Err(ContractError::InvalidParameter {
            param: "amount".into(),
        });
    };

    // Verify that the sender is the owner of the token
    let token_owner = Cw721Contract(collection.clone())
        .owner_of(&deps.querier, token_id.to_string(), true)?
        .owner;
    if info.sender != token_owner {
        return Err(ContractError::Unauthorized {});
    };

    // Verify that the token is not already wagered or is not matchmaking
    let token_status = query_token_status(deps.as_ref(), token.clone())?.token_status;
    if token_status != TokenStatus::None {
        return Err(ContractError::AlreadyWagered {});
    };

    // Search for a MatchmakingItem in MATCHMAKING that matches any of the currencies in `against_currencies`.
    // This MatchmakingItem must also match the expiry and amount.
    // If a MatchmakingItem is found, then the token is matched with the token in the MatchmakingItem and a Wager is created.
    // If a MatchmakingItem is not found, then a MatchmakingItem is created with the token and the other parameters.

    let matchmaking_item = MATCHMAKING
        .range(deps.storage, None, None, Order::Ascending)
        .find(|item| {
            item.as_ref()
                .map(|(_, v)| {
                    v.against_currencies.contains(&currency)
                        && against_currencies.contains(&v.currency)
                        && v.expiry == expiry
                        && v.amount == amount
                        && v.expires_at > env.block.time
                })
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            Err(cosmwasm_std::StdError::NotFound {
                kind: "matchmaking_item".into(),
            })
        });

    if matchmaking_item.is_ok() {
        let (
            matchmaking_key,
            MatchmakingItem {
                currency: match_currency,
                ..
            },
        ) = matchmaking_item?;

        let (_, token_id) = matchmaking_key;

        let against_token: Token = (collection, token_id);
        let expires_at = env.block.time.plus_seconds(expiry);

        let wager = Wager {
            currencies: (currency, match_currency),
            expires_at,
            amount,
        };

        WAGERS.save(deps.storage, (token.clone(), against_token), &wager)?;

        MATCHMAKING.remove(deps.storage, matchmaking_key);

        Ok(Response::new()
            .add_attribute("action", "wager")
            .add_attribute("collection", token.0)
            .add_attribute("token_id", token.1.to_string())
            .add_attribute("expires_at", expires_at.to_string()))
    } else {
        let expires_at = env.block.time.plus_seconds(config.matchmaking_expiry);
        let matchmaking_item = MatchmakingItem {
            expires_at,
            currency,
            against_currencies,
            expiry,
            amount,
        };

        MATCHMAKING.save(deps.storage, token.clone(), &matchmaking_item)?;

        Ok(Response::new()
            .add_attribute("action", "matchmake")
            .add_attribute("collection", token.0)
            .add_attribute("token_id", token.1.to_string())
            .add_attribute("expires_at", expires_at.to_string()))
    }
}

pub fn execute_cancel(
    deps: DepsMut,
    info: MessageInfo,
    token: Token,
) -> Result<Response, ContractError> {
    let (collection, token_id) = token.clone();

    // Verify that the token's collection address is the same as the contract address in config
    let config = CONFIG.load(deps.storage)?;
    if collection != config.collection_address {
        return Err(ContractError::Unauthorized {});
    }

    // Verify that the sender is the owner of the token
    let token_owner = Cw721Contract(collection)
        .owner_of(&deps.querier, token_id.to_string(), true)?
        .owner;
    if info.sender != token_owner {
        return Err(ContractError::Unauthorized {});
    };

    let token_status = query_token_status(deps.as_ref(), token.clone())?.token_status;

    match token_status {
        TokenStatus::Matchmaking(status) => {
            MATCHMAKING.remove(deps.storage, token.clone());
            let msg = send_tokens(info.sender, coin(status.amount.u128(), NATIVE_DENOM))?;
            Ok(Response::new()
                .add_submessage(msg)
                .add_attribute("action", "cancel")
                .add_attribute("collection", token.0)
                .add_attribute("token_id", token.1.to_string()))
        }
        _ => Err(ContractError::NotMatchmaking {}),
    }
}

pub fn execute_set_winner(
    deps: DepsMut,
    env: Env,
    wager_key: (Token, Token),
    winner: Token,
) -> Result<Response, ContractError> {
    let (collection, token_id) = winner.clone();

    // Verify that all token data conforms to the contract's config
    let config = CONFIG.load(deps.storage)?;
    if wager_key.0 .0 != config.collection_address || wager_key.1 .0 != config.collection_address {
        return Err(ContractError::Unauthorized {});
    }

    // Verify that the winner is part of the wager key
    if winner != wager_key.0 && winner != wager_key.1 {
        return Err(ContractError::InvalidParameter {
            param: "winner".into(),
        });
    }

    // Get the wager info
    let wager = WAGERS.load(deps.storage, wager_key.clone())?;
    let token_owner = Cw721Contract(collection)
        .owner_of(&deps.querier, token_id.to_string(), true)?
        .owner;

    // Verify that the wager has expired
    if env.block.time < wager.expires_at {
        return Err(ContractError::WagerActive {});
    }

    // Remove the wager and pay out the winner

    WAGERS.remove(deps.storage, wager_key);

    let wager_total = wager.amount * Uint128::from(2u128);

    let app_fee = wager_total * config.fee_percent / Uint128::from(100u128);
    let fairburn_fee = wager_total * config.fairburn_percent / Uint128::from(100u128);

    let winner_amount = wager_total - app_fee - fairburn_fee;

    // Charge fee & fair burn
    let mut res = Response::new().add_attribute("action", "set_winner");

    let fee_msg = send_tokens(
        config.fee_address.clone(),
        coin(app_fee.u128(), NATIVE_DENOM),
    )?;
    let winner_msg = send_tokens(
        deps.api.addr_validate(&token_owner)?,
        coin(winner_amount.u128(), NATIVE_DENOM),
    )?;

    fair_burn(fairburn_fee.u128(), Some(config.fee_address), &mut res);

    Ok(res.add_submessages(vec![fee_msg, winner_msg]))
}

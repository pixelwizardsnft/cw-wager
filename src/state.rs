use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum Currency {
    DOT,
    AVAX,
    UNI,
    ATOM,
    LINK,
    NEAR,
    ICP,
    SAND,
    BTC,
    ETH,
    BNB,
    XRP,
    ADA,
    DOGE,
    SOL,
    MANA,
    CAKE,
    AR,
    OSMO,
    RUNE,
    LUNA,
    USTC,
    STARS,
    MIR,
}

#[cw_serde]
pub struct Wager {
    pub currencies: (Currency, Currency),
    pub amount: Uint128,
    pub expires_at: Timestamp,
}

#[cw_serde]
pub struct NFT {
    pub collection: Addr,
    pub token_id: u64,
}

#[cw_serde]
pub struct WagerInfo {
    pub token: NFT,
    pub currency: Currency,
}

#[cw_serde]
pub struct WagerExport {
    pub amount: Uint128,
    pub expires_at: Timestamp,
    pub wagers: (WagerInfo, WagerInfo),
}

#[cw_serde]
pub struct MatchmakingItem {
    pub currency: Currency,
    pub against_currencies: Vec<Currency>,
    pub expires_at: Timestamp, // when this expires, remove it
    pub expiry: u64,           // expiry of the wager in seconds
    pub amount: Uint128,
}

#[cw_serde]
pub struct MatchmakingItemExport {
    pub token: NFT,
    pub currency: Currency,
    pub against_currencies: Vec<Currency>,
    pub expires_at: Timestamp, // when this expires, remove it
    pub expiry: u64,           // expiry of the wager in seconds
    pub amount: Uint128,
}

#[cw_serde]
pub enum TokenStatus {
    Matchmaking(MatchmakingItemExport),
    Wager(WagerExport),
    None,
}

pub type Token = (Addr, u64);
pub type WagerKey = (Token, Token);

pub const WAGERS: Map<WagerKey, Wager> = Map::new("wagers");
pub const MATCHMAKING: Map<Token, MatchmakingItem> = Map::new("matchmaking");

#[cw_serde]
pub struct Config {
    // Max amount of currencies that can be wagered against when matchmaking
    pub max_currencies: u8,
    // List of wager amount options (ex: 50,100,250 STARS)
    pub amounts: Vec<Uint128>,
    // List of wager expiry options in seconds (ex: 900,1800,3600)
    pub expiries: Vec<u64>,
    // Percentage of the wager amount that goes to the fee collector
    pub fee_percent: Decimal,
    // Percentage of the wager amount that is fair burned
    pub fairburn_percent: Decimal,
    // Address that receives the fee
    pub fee_address: Addr,
    // Address of the NFT collection
    pub collection_address: Addr,
    // Time in seconds before a matchmaking item expires
    pub matchmaking_expiry: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

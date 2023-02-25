use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

#[cw_serde]
pub enum Currency {
    Dot,
    Avax,
    Uni,
    Atom,
    Link,
    Near,
    Icp,
    Sand,
    Btc,
    Eth,
    Bnb,
    Xrp,
    Ada,
    Doge,
    Sol,
    Mana,
    Cake,
    Ar,
    Osmo,
    Rune,
    Luna,
    Ustc,
    Stars,
    Mir,
}

#[cw_serde]
pub struct Wager {
    pub id: WagerKey,
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

pub struct WagerIndicies<'a> {
    pub id: UniqueIndex<'a, WagerKey, Wager>,
}

impl<'a> IndexList<Wager> for WagerIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Wager>> + '_> {
        let v: Vec<&dyn Index<Wager>> = vec![&self.id];
        Box::new(v.into_iter())
    }
}

pub fn wagers<'a>() -> IndexedMap<'a, WagerKey, Wager, WagerIndicies<'a>> {
    let indexes = WagerIndicies {
        id: UniqueIndex::new(|d| d.id.clone(), "wager_id"),
    };
    IndexedMap::new("bids", indexes)
}

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

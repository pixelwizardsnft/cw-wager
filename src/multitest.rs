#[cfg(test)]
use cosmwasm_std::{coin, Timestamp, Uint128};
use cosmwasm_std::{coins, Addr, Coin};
use cw721::Cw721ExecuteMsg;
use sg2::tests::mock_collection_params_1;

use cw_multi_test::{BankSudo, Contract, ContractWrapper, Executor, SudoMsg as CwSudoMsg};
use sg_multi_test::StargazeApp;

use sg_std::{StargazeMsgWrapper, GENESIS_MINT_START_TIME, NATIVE_DENOM};
use vending_factory::msg::{
    ExecuteMsg as VendingFactoryExecuteMsg, VendingMinterCreateMsg, VendingMinterInitMsgExtension,
};
use vending_factory::state::{ParamsExtension, VendingMinterParams};
use vending_factory::{helpers::FactoryContract, msg::InstantiateMsg as FactoryInstantiateMsg};

use crate::config::ParamInfo;
use crate::msg::{ConfigResponse, ExecuteMsg, QueryMsg, WagersResponse};
use crate::ContractError;

const GOVERNANCE: &str = "governance";
const ADMIN: &str = "admin";
const CREATOR: &str = "creator";
const TOKEN1_ID: u32 = 45;
const TOKEN2_ID: u32 = 85;

const SENDER: &str = "sender";
const PEER: &str = "peer";

fn custom_mock_app() -> StargazeApp {
    StargazeApp::default()
}

pub const CREATION_FEE: u128 = 5_000_000_000;
// pub const MINT_PRICE: u128 = 100_000_000;

pub const MAX_TOKEN_LIMIT: u32 = 10000;

pub const MIN_MINT_PRICE: u128 = 50_000_000;
pub const AIRDROP_MINT_PRICE: u128 = 0;
pub const MINT_FEE_FAIR_BURN: u64 = 1_000; // 10%
pub const AIRDROP_MINT_FEE_FAIR_BURN: u64 = 10_000; // 100%
pub const SHUFFLE_FEE: u128 = 500_000_000;
pub const MAX_PER_ADDRESS_LIMIT: u32 = 50;

pub fn mock_params() -> VendingMinterParams {
    VendingMinterParams {
        code_id: 1,
        creation_fee: coin(CREATION_FEE, NATIVE_DENOM),
        min_mint_price: coin(MIN_MINT_PRICE, NATIVE_DENOM),
        mint_fee_bps: MINT_FEE_FAIR_BURN,
        max_trading_offset_secs: 60 * 60 * 24 * 7,
        extension: ParamsExtension {
            max_token_limit: MAX_TOKEN_LIMIT,
            max_per_address_limit: MAX_PER_ADDRESS_LIMIT,
            airdrop_mint_price: coin(AIRDROP_MINT_PRICE, NATIVE_DENOM),
            airdrop_mint_fee_bps: AIRDROP_MINT_FEE_FAIR_BURN,
            shuffle_fee: coin(SHUFFLE_FEE, NATIVE_DENOM),
        },
    }
}

pub fn mock_init_extension(
    splits_addr: Option<String>,
    start_time: Option<Timestamp>,
) -> VendingMinterInitMsgExtension {
    vending_factory::msg::VendingMinterInitMsgExtension {
        base_token_uri: "ipfs://aldkfjads".to_string(),
        payment_address: splits_addr,
        start_time: start_time.unwrap_or(Timestamp::from_nanos(GENESIS_MINT_START_TIME)),
        num_tokens: 100,
        mint_price: coin(MIN_MINT_PRICE, NATIVE_DENOM),
        per_address_limit: 5,
        whitelist: None,
    }
}

pub fn contract_cw_wager() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_sg721() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_vending_factory() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_factory::contract::execute,
        vending_factory::contract::instantiate,
        vending_factory::contract::query,
    );
    Box::new(contract)
}

pub fn contract_vending_minter() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_minter::contract::execute,
        vending_minter::contract::instantiate,
        vending_minter::contract::query,
    )
    .with_reply(vending_minter::contract::reply);
    Box::new(contract)
}

fn setup_block_time(router: &mut StargazeApp, seconds: u64) {
    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(seconds);
    router.set_block(block);
}

fn setup_contracts(
    router: &mut StargazeApp,
    creator: &Addr,
) -> Result<(Addr, Addr), ContractError> {
    let factory_id = router.store_code(contract_vending_factory());
    let minter_id = router.store_code(contract_vending_minter());

    let mut params = mock_params();
    params.code_id = minter_id;

    let msg = FactoryInstantiateMsg { params };
    let factory_addr = router
        .instantiate_contract(
            factory_id,
            Addr::unchecked(GOVERNANCE),
            &msg,
            &[],
            "factory",
            Some(GOVERNANCE.to_string()),
        )
        .unwrap();

    let factory_contract = FactoryContract(factory_addr);

    let sg721_id = router.store_code(contract_sg721());

    let collection_params =
        mock_collection_params_1(Some(Timestamp::from_nanos(GENESIS_MINT_START_TIME)));
    let mut m = VendingMinterCreateMsg {
        init_msg: mock_init_extension(None, None),
        collection_params,
    };
    m.collection_params.code_id = sg721_id;
    let msg = VendingFactoryExecuteMsg::CreateMinter(m);

    let creation_fee = coin(CREATION_FEE, NATIVE_DENOM);

    router
        .sudo(CwSudoMsg::Bank(BankSudo::Mint {
            to_address: ADMIN.to_string(),
            amount: vec![creation_fee.clone()],
        }))
        .unwrap();

    let bal = router.wrap().query_all_balances(ADMIN).unwrap();
    assert_eq!(bal, vec![creation_fee.clone()]);

    // this should create the minter + sg721
    let cosmos_msg = factory_contract.call_with_funds(msg, creation_fee).unwrap();

    let res = router.execute(Addr::unchecked(ADMIN), cosmos_msg);
    assert!(res.is_ok());

    // Instantiate wager contract
    let cw_wager_id = router.store_code(contract_cw_wager());
    let msg = crate::msg::InstantiateMsg {
        max_currencies: 3,
        amounts: vec![
            Uint128::from(100_000_000u128),
            Uint128::from(250_000_000u128),
            Uint128::from(500_000_000u128),
        ],
        expiries: vec![15, 30, 60],
        fee_bps: 400,      // 4%
        fairburn_bps: 100, // 1%
        fee_address: CREATOR.into(),
        collection_address: Addr::unchecked("contract2").to_string(),
        matchmaking_expiry: 60,
    };
    let cw_wager = router
        .instantiate_contract(
            cw_wager_id,
            creator.clone(),
            &msg,
            &[],
            "cw-wager",
            Some(CREATOR.to_string()),
        )
        .unwrap();

    Ok((cw_wager, Addr::unchecked("contract2")))
}

// Intializes accounts with balances
fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
    let sender: Addr = Addr::unchecked(SENDER);
    let peer: Addr = Addr::unchecked(PEER);
    let creator: Addr = Addr::unchecked(CREATOR);

    let creator_funds: Vec<Coin> = coins(1_000_000_000_000_000, NATIVE_DENOM);
    let funds: Vec<Coin> = coins(2_000_000_000_000_000, NATIVE_DENOM);

    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: sender.to_string(),
                amount: funds.clone(),
            }
        }))
        .ok();
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: peer.to_string(),
                amount: funds.clone(),
            }
        }))
        .ok();
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: creator.to_string(),
                amount: creator_funds.clone(),
            }
        }))
        .ok();

    // Check native balances
    let owner_native_balances = router.wrap().query_all_balances(sender.clone()).unwrap();
    assert_eq!(owner_native_balances, funds);
    let bidder_native_balances = router.wrap().query_all_balances(peer.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, creator_funds);

    Ok((sender, peer, creator))
}

#[test]
fn try_update_config() {
    let router = &mut custom_mock_app();

    let (sender, _, creator) = setup_accounts(router).unwrap();
    let (wager_contract, _) = setup_contracts(router, &creator).unwrap();

    // This will only update max_currencies
    let update_config_msg = ExecuteMsg::UpdateConfig {
        params: ParamInfo {
            max_currencies: Some(2),
            amounts: None,
            expiries: None,
            fee_bps: None,
            fairburn_bps: None,
            fee_address: None,
            collection_address: None,
            matchmaking_expiry: None,
        },
    };

    // Attempt to update config from non-admin address
    // Expects: failure
    let err = router
        .execute_contract(
            sender.clone(),
            wager_contract.clone(),
            &update_config_msg,
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Attempt to update config from admin address
    // Expects: success
    let res = router.execute_contract(
        creator.clone(),
        wager_contract.clone(),
        &update_config_msg,
        &[],
    );
    assert!(res.is_ok());

    // Query new config
    let config_query_msg = QueryMsg::Config {};
    let config_response: ConfigResponse = router
        .wrap()
        .query_wasm_smart(wager_contract.clone(), &config_query_msg)
        .unwrap();

    assert_eq!(config_response.config.max_currencies, 2);
}

#[test]
fn try_wager() {
    // TODO: Incomplete coverage, only covering success
    let router = &mut custom_mock_app();

    let (sender, peer, creator) = setup_accounts(router).unwrap();
    let (wager_contract, collection) = setup_contracts(router, &creator).unwrap();

    setup_block_time(
        router,
        Timestamp::from_nanos(GENESIS_MINT_START_TIME).seconds() + 100,
    );

    // Mint nfts 45 & 85
    let mint_msg = vending_minter::msg::ExecuteMsg::Mint {};
    let res = router.execute_contract(
        creator.clone(),
        Addr::unchecked("contract1").clone(),
        &mint_msg,
        &[coin(MIN_MINT_PRICE, NATIVE_DENOM)],
    );
    assert!(res.is_ok());
    let res = router.execute_contract(
        creator.clone(),
        Addr::unchecked("contract1").clone(),
        &mint_msg,
        &[coin(MIN_MINT_PRICE, NATIVE_DENOM)],
    );
    assert!(res.is_ok());

    // Transfer nfts to sender & peer
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: sender.to_string(),
        token_id: TOKEN1_ID.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: peer.to_string(),
        token_id: TOKEN2_ID.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());

    // Submit a wager for matchmaking
    let wager_msg = ExecuteMsg::Wager {
        token: (collection.clone(), TOKEN1_ID as u64),
        currency: crate::state::Currency::ATOM,
        against_currencies: vec![crate::state::Currency::STARS],
        expiry: 60,
    };

    // Attempt to submit a wager from `sender`
    // Expects: success
    let res = router.execute_contract(
        sender.clone(),
        wager_contract.clone(),
        &wager_msg,
        &[coin(100_000_000, NATIVE_DENOM)],
    );
    assert!(res.is_ok());

    // Submit a wager for matchmaking to `sender` from `peer`
    let wager_msg = ExecuteMsg::Wager {
        token: (collection.clone(), TOKEN2_ID as u64),
        currency: crate::state::Currency::STARS,
        against_currencies: vec![crate::state::Currency::ATOM],
        expiry: 60,
    };

    // Attempt to submit a wager from `peer`
    // Expects: success
    let res = router.execute_contract(
        peer.clone(),
        wager_contract.clone(),
        &wager_msg,
        &[coin(100_000_000, NATIVE_DENOM)],
    );
    assert!(res.is_ok());

    let query_msg = QueryMsg::Wagers {};
    let res: WagersResponse = router
        .wrap()
        .query_wasm_smart(wager_contract.clone(), &query_msg)
        .unwrap();
    println!("{:?}", res);

    // Attempt to set the wager as won, even thought it has not expired yet
    // Expects: failure
    let set_winner_msg = ExecuteMsg::SetWinner {
        wager_key: (
            (collection.clone(), TOKEN1_ID as u64),
            (collection.clone(), TOKEN2_ID as u64),
        ),
        winner: (collection.clone(), TOKEN2_ID as u64),
    };
    let err = router
        .execute_contract(
            creator.clone(),
            wager_contract.clone(),
            &set_winner_msg,
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::WagerActive {}
    );

    // Set time further into the future
    setup_block_time(
        router,
        Timestamp::from_nanos(GENESIS_MINT_START_TIME).seconds() + 1000,
    );

    // Attempt to set the wager as won
    // Expects: success
    let res = router.execute_contract(
        creator.clone(),
        wager_contract.clone(),
        &set_winner_msg,
        &[],
    );
    println!("{:?}", res);
    assert!(res.is_ok());
}

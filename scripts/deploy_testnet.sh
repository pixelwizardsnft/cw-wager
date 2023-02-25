# Contract deploy script
# Run it like this: `zsh ./scripts/deploy_testnet.sh`

# View your keys with `starsd keys list`

export CONTRACT_NAME=cw_wager;
export KEY_NAME=testnet;

export WALLET_DATA=$(starsd keys show $KEY_NAME --output json | jq .);

export KEY_NAME=$(echo $WALLET_DATA | jq -r '.name');
export KEY_TYPE=$(echo $WALLET_DATA | jq -r '.type');
export KEY_ADDRESS=$(echo $WALLET_DATA | jq -r '.address');

echo "\nConnected to wallet '$KEY_NAME'<$KEY_TYPE> @ $KEY_ADDRESS";
echo "\n========\n";

# Instantiate message config
export INSTANTIATE_MSG="{\"max_currencies\": 3, \"amounts\": [\"100000000\", \"250000000\", \"500000000\"], \"expiries\": [600, 900, 1800], \"fee_bps\": 400, \"fairburn_bps\": 100,  \"fee_address\": \"$KEY_ADDRESS\", \"collection_address\": \"stars1eljycn4sljw0da0yfqtpcfprqhrq3j27d7vwfe5spmeytls7lh8s2upw90\", \"matchmaking_expiry\": 900}";
# echo $INSTANTIATE_MSG;

## INIT ##
# Get network config
echo "Sourcing network configuration...";

export NODE="https://rpc.elgafar-1.stargaze-apis.com:443"

# Set starsd config
starsd config chain-id "elgafar-1";
starsd config node $NODE;

# Tx flag configuration
export TXFLAG=(--gas-prices 0.25ustars --gas auto --gas-adjustment 1.3);

echo "Network configuration found."

## BUILD ##
# If the architecture is `arm64`, run the arm64 version of rust-optimizer
echo "\n========\n";
echo "Building contract...";

export ARCH='';
export L_ARCH='';

if [[ $(uname -m) -eq 'arm64' ]]
then
  ARCH='-arm64';
  LARCH='-aarch64';
fi

docker run --rm -v "$(pwd)":/code \
--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
cosmwasm/rust-optimizer$ARCH:0.12.8;

CONTRACT_NAME=$CONTRACT_NAME$LARCH;

## DEPLOY ##

# Fetch codeid
echo "\n========\n";
echo "Fetching CodeIDs...";

export RES=$(starsd tx wasm store artifacts/$CONTRACT_NAME.wasm --from $KEY_NAME --gas-prices 0.25ustars --gas auto --gas-adjustment 1.3 -y --output json -b block);
export CODE_ID=$(echo $RES | jq -r '.logs[0].events[1].attributes[1].value');
echo "CodeID found: $CODE_ID";

# Instantiate the contract
echo "\n========\n";
echo "Instantiating contract...";
starsd tx wasm instantiate $CODE_ID "$INSTANTIATE_MSG" --from $KEY_NAME --label "$CONTRACT_NAME"  --gas-prices 0.25ustars --gas auto --gas-adjustment 1.3 -y --no-admin;
echo "Contract instantiated."

# Store contract addr in $CONTRACT
echo "\n========\n";
echo "Fetching contract address...";
sleep 6;
export CONTRACT=$(starsd query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]');
echo "Contract address: $fg_bold[green]$CONTRACT";
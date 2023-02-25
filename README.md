# PixelWizards wagering contract

Contract to allow the owners of two NFTs from a set collection to wager on an increase of the price of a specific asset over a set period of time.

### Matchmaking

Users submit their intentions to start a wager to the contract, which matches them if someone else meeting their specific conditions is found. If not, the user is added to the matchmaking pool, and their wager will be started when a suitable opponent is found. **The time in seconds for a matchmaking item to expire can be set in `InstantiateMsg`.**

<img width="1456" alt="Screenshot 2023-02-24 at 8 18 45 PM" src="https://user-images.githubusercontent.com/25516960/221328078-ca4fbe20-3c37-405f-afda-0568e96a329a.png">


### Fees
Contract fee and fee burn fee percentages can be set in `InstantiateMsg`.

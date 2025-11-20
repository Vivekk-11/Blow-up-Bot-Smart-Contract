
### Overview

This smart contract implements a pump.fun–style token launch mechanism on Solana, meaning it uses a bonding curve where token prices automatically rise as users buy, allows buying and selling during the curve phase, and automatically creates a Raydium liquidity pool (token–wSOL) once a predefined threshold is reached.

It recreates the essential pump.fun launch flow in a clean, Anchor-based on-chain program.

Integrated with the BlowUpBot Telegram bot, users can launch, buy, sell, and track tokens directly inside Telegram without needing a website or technical knowledge.

### Key Features

**Pump.fun–Style Bonding Curve**
- Token price automatically increases as more users buy.
- Users can buy and sell during the bonding curve phase.

**Automatic Liquidity on Raydium**
- Once the bonding curve threshold is hit, the contract automatically creates a Raydium liquidity pool (token–wSOL).
- After this, the token becomes publicly tradable on Raydium.

**Instant Token Creation**
- Launch a new Solana token with a single action.
- All required accounts, vaults, and metadata are set up automatically.

**Integrated with BlowUpBot (Telegram)**
- All launch actions (create token, buy, sell, etc.) work directly from Telegram.
- No website, no coding, no blockchain expertise needed.

**DCA (Dollar Cost Averaging) Support**
- The system is designed to support periodic token accumulation flows via the bot.

**Creator & Platform Fees**
- Optional fees built into buys/sells or LP migration.
- Allows founders & platforms to monetize token launches.

**Safety Controls**
- Ability to pause the launch or stop trading if required.

**Built with Anchor**
- Clean, secure, reliable Solana program architecture.

### Technical Overview (How Graduation Works)

To mirror the pump.fun launch flow, the contract includes an automated graduation mechanism that transitions a token from bonding-curve trading to Raydium liquidity.

Here’s how it works:

1. Every time the buy instruction is executed, the program checks whether the SOL reserves inside the bonding curve vault have reached (or exceeded) the graduation threshold defined at launch.
2. If the threshold is met, the program triggers an internal function called `graduate_internal`:
   - Re-checks all reserve values for safety.
   - Prepares and validates all required accounts.
   - Updates internal state.
   - Emits a `CreatePoolRequestEvent`.
3. An off-chain relayer listens to this event:
   - Converts the program’s SOL reserves to wrapped SOL (wSOL).
   - Creates a Raydium liquidity pool for the pair (token–wSOL).
   - Supplies the LP with the correct token + wSOL ratios.
   - Calls the program’s `graduate` instruction once LP creation is complete.
4. The `graduate` instruction finalizes the process:
   - Updates the bonding curve status to `Graduated`.
   - Permanently disables further buys/sells on the bonding curve.
   - Locks in final state for that launch.
   - Publishes an event confirming successful graduation.
5. After graduation, users can no longer trade via the bonding curve. All trading moves to Raydium’s AMM, where the token is now live and publicly tradable.


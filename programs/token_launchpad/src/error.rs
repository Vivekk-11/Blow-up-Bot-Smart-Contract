use anchor_lang::prelude::*;

#[error_code]
pub enum PumpError {
    #[msg("Not authorized address")]
    NotAuthorized,

    #[msg("Fee recipient address is not match with the one in the config")]
    IncorrectFeeRecipient,

    #[msg("Insufficient sol")]
    InsufficientSol,

    #[msg("The value is not in the expected range")]
    IncorrectValue,

    #[msg("Amount out is smaller than required amount")]
    ReturnAmountTooSmall,

    #[msg("An overflow or underflow occurred during the calculation")]
    OverflowOrUnderflowOccurred,

    #[msg("Curve is already completed")]
    CurveAlreadyCompleted,

    #[msg("Insufficent tokens")]
    InsufficientTokens,

    #[msg("Invalid")]
    MathOverflow,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Invalid Treasury")]
    InvalidTreasury,

    #[msg("Slippage exceeded")]
    SlippageExceeded,

    #[msg("Invalid Total Volume Sol")]
    InvalidTotalVolumeSol,

    #[msg("Invalid Real Sol Reserves")]
    InvalidRealSolReserves,

    #[msg("Invalid Real Token Reserves")]
    InvalidRealTokenReserves,

    #[msg("Invalid Virtual Sol Reserves")]
    InvalidVirtualSolReserves,

    #[msg("Invalid Virtual Token Reserves")]
    InvalidVirtualTokenReserves,

    #[msg("Invalid Constant")]
    InvalidConstant,

    #[msg("Problem with calculating sol reserves")]
    InvalidNewSolReserves,

    #[msg("Problem with calculating new token reserves")]
    InvalidNewTokenReserves,

    #[msg("Problem with calculating tokens out")]
    InvalidTokensCalculation,

    #[msg("Numeric Overflow")]
    NumericOverflow,

    #[msg("Divide By Zero")]
    DivideByZero,

    #[msg("Token Graduated")]
    TokenGraduated,

    #[msg("Token is not active")]
    TokenNotActive,
}

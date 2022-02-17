use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke,
    pubkey::Pubkey,
    sysvar,
};

use crate::helpers::flash_loan_proxy::FlashLoanProxyError::InvalidInstruction;
use spl_token::solana_program::{account_info::next_account_info, program_error::ProgramError};
use std::convert::TryInto;
use std::mem::size_of;
use thiserror::Error;

use spl_token_lending::{
    instruction::flash_borrow_reserve_liquidity, instruction::flash_repay_reserve_liquidity,
};

pub enum FlashLoanProxyInstruction {
    ProxyBorrow { liquidity_amount: u64 },
    ProxyRepay { liquidity_amount: u64 },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, instruction_data)
}

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = FlashLoanProxyInstruction::unpack(instruction_data)?;

        match instruction {
            FlashLoanProxyInstruction::ProxyBorrow { liquidity_amount } => {
                msg!("Instruction: Proxy Borrow");
                Self::process_proxy_borrow(accounts, liquidity_amount, program_id)
            }
            FlashLoanProxyInstruction::ProxyRepay { liquidity_amount } => {
                msg!("Instruction: Proxy Repay");
                Self::process_proxy_repay(accounts, liquidity_amount, program_id)
            }
        }
    }

    fn process_proxy_repay(
        accounts: &[AccountInfo],
        liquidity_amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_liquidity_info = next_account_info(account_info_iter)?;
        let destination_liquidity_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_fee_receiver_info = next_account_info(account_info_iter)?;
        let host_fee_receiver_info = next_account_info(account_info_iter)?;
        let reserve_info = next_account_info(account_info_iter)?;
        let token_lending_info = next_account_info(account_info_iter)?;
        let lending_market_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;

        invoke(
            &flash_repay_reserve_liquidity(
                *token_lending_info.key,
                liquidity_amount,
                *source_liquidity_info.key,
                *destination_liquidity_info.key,
                *reserve_liquidity_fee_receiver_info.key,
                *host_fee_receiver_info.key,
                *reserve_info.key,
                *lending_market_info.key,
                *user_transfer_authority_info.key,
            ),
            accounts,
        )?;

        Ok(())
    }

    fn process_proxy_borrow(
        accounts: &[AccountInfo],
        liquidity_amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_liquidity_info = next_account_info(account_info_iter)?;
        let destination_liquidity_info = next_account_info(account_info_iter)?;
        let reserve_info = next_account_info(account_info_iter)?;
        let token_lending_info = next_account_info(account_info_iter)?;
        let lending_market_info = next_account_info(account_info_iter)?;

        invoke(
            &flash_borrow_reserve_liquidity(
                *token_lending_info.key,
                liquidity_amount,
                *source_liquidity_info.key,
                *destination_liquidity_info.key,
                *reserve_info.key,
                *lending_market_info.key,
            ),
            accounts,
        )?;

        Ok(())
    }
}

impl FlashLoanProxyInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::ProxyBorrow {
                liquidity_amount: Self::unpack_amount(rest)?,
            },
            1 => Self::ProxyRepay {
                liquidity_amount: Self::unpack_amount(rest)?,
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let liquidity_amount = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(liquidity_amount)
    }
}

#[derive(Error, Debug, Copy, Clone)]
pub enum FlashLoanProxyError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("The account is not currently owned by the program")]
    IncorrectProgramId,
}

impl From<FlashLoanProxyError> for ProgramError {
    fn from(e: FlashLoanProxyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// Creates a 'RepayProxy' instruction.
#[allow(clippy::too_many_arguments)]
pub fn repay_proxy(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_liquidity_fee_receiver_pubkey: Pubkey,
    host_fee_receiver_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    token_lending_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_liquidity_pubkey, false),
            AccountMeta::new(reserve_liquidity_fee_receiver_pubkey, false),
            AccountMeta::new(host_fee_receiver_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new_readonly(token_lending_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: FlashLoanProxyInstruction::ProxyRepay { liquidity_amount }.pack(),
    }
}

/// Creates a 'BorrowProxy' instruction.
#[allow(clippy::too_many_arguments)]
pub fn borrow_proxy(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    token_lending_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_liquidity_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new_readonly(token_lending_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: FlashLoanProxyInstruction::ProxyBorrow { liquidity_amount }.pack(),
    }
}

impl FlashLoanProxyInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::ProxyBorrow { liquidity_amount } => {
                buf.push(0);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::ProxyRepay { liquidity_amount } => {
                buf.push(1);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
        }
        buf
    }
}

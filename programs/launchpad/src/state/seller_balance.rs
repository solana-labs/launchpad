use anchor_lang::prelude::*;

#[account]
#[derive(Default, Debug)]
pub struct SellerBalance {
    pub owner: Pubkey,
    pub custody: Pubkey,
    pub balance: u64,
    pub bump: u8,
}

impl SellerBalance {
    pub const LEN: usize = 8 + std::mem::size_of::<SellerBalance>();
}

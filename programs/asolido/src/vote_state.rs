use crate::LidoError;
use solana_program::pubkey::Pubkey;
use solana_program::vote::program::ID;
use std::convert::TryInto;
use anchor_lang::error;

/// Structure used to read the first 4 fields of a Solana `VoteAccount`.
/// The original `VoteAccount` structure cannot be used in a Solana
/// program due to size constrains.

#[derive(Clone)]
pub struct PartialVoteState {
    /// comes from an enum inside the `VoteState` structure
    pub version: u32,
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for withdrawals
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    ///  payout should be given to this VoteAccount
    pub commission: u8,
}

impl anchor_lang::AccountDeserialize for PartialVoteState {
    fn try_deserialize_unchecked(data: &mut &[u8]) -> anchor_lang::Result<Self> {
        // Read 4 bytes for u32.
        let version = u32::from_le_bytes(
            data[0..4]
                .try_into()
                .map_err(|_| error!(LidoError::InvalidVoteAccount))?,
        );

        let mut pubkey_buf: [u8; 32] = Default::default();
        // Read 32 bytes for Pubkey.
        pubkey_buf.copy_from_slice(&data[4..][..32]);
        let node_pubkey = Pubkey::new_from_array(pubkey_buf);
        // Read 32 bytes for Pubkey.
        pubkey_buf.copy_from_slice(&data[36..][..32]);
        let authorized_withdrawer = Pubkey::new_from_array(pubkey_buf);

        // Read 1 byte for u8.
        let commission = data[68];

        Ok(PartialVoteState {
            version,
            node_pubkey,
            authorized_withdrawer,
            commission,
        })
    }
}

impl anchor_lang::AccountSerialize for PartialVoteState {}

impl anchor_lang::Owner for PartialVoteState {
    fn owner() -> Pubkey {
        ID
    }
}

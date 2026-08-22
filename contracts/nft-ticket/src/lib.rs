#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

// 1. Error Handling khusus Soroban
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u32)]
pub enum TicketError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    SoldOut = 3,
    AlreadyUsed = 4,
    TicketNotFound = 5,
}

// 2. Key untuk Storage
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    MaxSupply,
    TotalMinted,
    Ticket(u32), // Menyimpan tiket berdasarkan Ticket ID
}

// 3. Struct Data Tiket
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ticket {
    pub owner: Address,
    pub is_used: bool,
}

#[contract]
pub struct NftTicketContract;

#[contractimpl]
impl NftTicketContract {
    // Inisialisasi kontrak: Set Admin dan Max Supply tiket
    pub fn initialize(env: Env, admin: Address, max_supply: u32) -> Result<(), TicketError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(TicketError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::MaxSupply, &max_supply);
        env.storage().instance().set(&DataKey::TotalMinted, &0u32);

        Ok(())
    }

    // Fungsi Minting Tiket Baru
    pub fn mint_ticket(env: Env, to: Address) -> Result<u32, TicketError> {
        to.require_auth();

        let max_supply: u32 = env
            .storage()
            .instance()
            .get(&DataKey::MaxSupply)
            .ok_or(TicketError::NotInitialized)?;

        let mut total_minted: u32 = env
            .storage()
            .instance()
            .get(&DataKey::TotalMinted)
            .unwrap_or(0);

        if total_minted >= max_supply {
            return Err(TicketError::SoldOut);
        }

        total_minted += 1;
        let ticket_id = total_minted;

        let new_ticket = Ticket {
            owner: to,
            is_used: false,
        };

        // Simpan data tiket dan perbarui total minted
        env.storage().persistent().set(&DataKey::Ticket(ticket_id), &new_ticket);
        env.storage().instance().set(&DataKey::TotalMinted, &total_minted);

        Ok(ticket_id)
    }

    // Fungsi Redeem / Pakai Tiket
    pub fn use_ticket(env: Env, ticket_id: u32) -> Result<(), TicketError> {
        let mut ticket: Ticket = env
            .storage()
            .persistent()
            .get(&DataKey::Ticket(ticket_id))
            .ok_or(TicketError::TicketNotFound)?;

        // Verifikasi bahwa pemanggil fungsi adalah pemilik tiket
        ticket.owner.require_auth();

        if ticket.is_used {
            return Err(TicketError::AlreadyUsed);
        }

        ticket.is_used = true;
        env.storage().persistent().set(&DataKey::Ticket(ticket_id), &ticket);

        Ok(())
    }

    // Fungsi Read-Only untuk Cek Tiket
    pub fn get_ticket(env: Env, ticket_id: u32) -> Option<Ticket> {
        env.storage().persistent().get(&DataKey::Ticket(ticket_id))
    }
}
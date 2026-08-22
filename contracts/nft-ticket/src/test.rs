use crate::{process_instruction, Ticket, TicketInstruction};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};

fn program_test(program_id: Pubkey) -> ProgramTest {
    ProgramTest::new("nft_ticket", program_id, processor!(process_instruction))
}

fn initialize_event_ix(
    program_id: Pubkey,
    organizer: Pubkey,
    event_id: u64,
    max_tickets: u32,
    price_lamports: u64,
) -> (Instruction, Pubkey) {
    let (event_pda, _bump) =
        Pubkey::find_program_address(&[b"event", &event_id.to_le_bytes()], &program_id);

    let data = TicketInstruction::InitializeEvent {
        event_id,
        max_tickets,
        price_lamports,
    }
    .try_to_vec()
    .unwrap();

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(organizer, true),
            AccountMeta::new(event_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    };
    (ix, event_pda)
}

fn issue_ticket_ix(
    program_id: Pubkey,
    payer: Pubkey,
    event_pda: Pubkey,
    mint: Pubkey,
    buyer_ata: Pubkey,
) -> (Instruction, Pubkey) {
    let (ticket_pda, _bump) =
        Pubkey::find_program_address(&[b"ticket", mint.as_ref()], &program_id);
    let (mint_authority, _bump) =
        Pubkey::find_program_address(&[b"mint_authority", event_pda.as_ref()], &program_id);

    let data = TicketInstruction::IssueTicket.try_to_vec().unwrap();

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(event_pda, false),
            AccountMeta::new(mint, true),
            AccountMeta::new(buyer_ata, false),
            AccountMeta::new(ticket_pda, false),
            AccountMeta::new_readonly(mint_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };
    (ix, ticket_pda)
}

fn check_in_ix(
    program_id: Pubkey,
    holder: Pubkey,
    holder_ata: Pubkey,
    mint: Pubkey,
    ticket_pda: Pubkey,
) -> Instruction {
    let data = TicketInstruction::CheckInTicket.try_to_vec().unwrap();
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(holder, true),
            AccountMeta::new(holder_ata, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(ticket_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

/// Shared setup: funds an organizer + buyer and initializes one event with
/// a max of 100 tickets. Returns everything a test needs to issue tickets.
async fn setup() -> (
    solana_program_test::BanksClient,
    Keypair, // fee payer / default test wallet
    solana_sdk::hash::Hash,
    Pubkey, // program_id
    Keypair, // organizer
    Keypair, // buyer
    Pubkey, // event_pda
) {
    let program_id = Pubkey::new_unique();
    let (mut banks_client, payer, recent_blockhash) = program_test(program_id).start().await;

    let organizer = Keypair::new();
    let buyer = Keypair::new();

    for kp in [&organizer, &buyer] {
        let tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(&payer.pubkey(), &kp.pubkey(), 10_000_000_000)],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        banks_client.process_transaction(tx).await.unwrap();
    }

    let (init_ix, event_pda) = initialize_event_ix(program_id, organizer.pubkey(), 1, 100, 1_000_000);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&organizer.pubkey()),
        &[&organizer],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    (banks_client, payer, recent_blockhash, program_id, organizer, buyer, event_pda)
}

#[tokio::test]
async fn issue_and_check_in_ticket() {
    let (mut banks_client, _payer, recent_blockhash, program_id, _organizer, buyer, event_pda) =
        setup().await;

    let mint = Keypair::new();
    let buyer_ata = get_associated_token_address(&buyer.pubkey(), &mint.pubkey());
    let create_ata_ix = create_associated_token_account(
        &buyer.pubkey(),
        &buyer.pubkey(),
        &mint.pubkey(),
        &spl_token::id(),
    );
    let (issue_ix, ticket_pda) =
        issue_ticket_ix(program_id, buyer.pubkey(), event_pda, mint.pubkey(), buyer_ata);

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, issue_ix],
        Some(&buyer.pubkey()),
        &[&buyer, &mint],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    // Ticket should exist, be unused, and the buyer should hold exactly 1 token.
    let ticket_account = banks_client.get_account(ticket_pda).await.unwrap().unwrap();
    let ticket = Ticket::try_from_slice(&ticket_account.data).unwrap();
    assert!(!ticket.used);
    assert_eq!(ticket.mint, mint.pubkey());

    // Check the ticket in.
    let checkin_ix = check_in_ix(program_id, buyer.pubkey(), buyer_ata, mint.pubkey(), ticket_pda);
    let tx = Transaction::new_signed_with_payer(
        &[checkin_ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    let ticket_account = banks_client.get_account(ticket_pda).await.unwrap().unwrap();
    let ticket = Ticket::try_from_slice(&ticket_account.data).unwrap();
    assert!(ticket.used);
}

#[tokio::test]
async fn double_check_in_fails() {
    let (mut banks_client, _payer, recent_blockhash, program_id, _organizer, buyer, event_pda) =
        setup().await;

    let mint = Keypair::new();
    let buyer_ata = get_associated_token_address(&buyer.pubkey(), &mint.pubkey());
    let create_ata_ix = create_associated_token_account(
        &buyer.pubkey(),
        &buyer.pubkey(),
        &mint.pubkey(),
        &spl_token::id(),
    );
    let (issue_ix, ticket_pda) =
        issue_ticket_ix(program_id, buyer.pubkey(), event_pda, mint.pubkey(), buyer_ata);

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, issue_ix],
        Some(&buyer.pubkey()),
        &[&buyer, &mint],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    let checkin_ix = check_in_ix(program_id, buyer.pubkey(), buyer_ata, mint.pubkey(), ticket_pda);
    let tx1 = Transaction::new_signed_with_payer(
        &[checkin_ix.clone()],
        Some(&buyer.pubkey()),
        &[&buyer],
        recent_blockhash,
    );
    banks_client.process_transaction(tx1).await.unwrap();

    // Second check-in must fail: the token was burned, so the holder's
    // account no longer has a balance of 1, and the ticket is already used.
    let tx2 = Transaction::new_signed_with_payer(
        &[checkin_ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        recent_blockhash,
    );
    let result = banks_client.process_transaction(tx2).await;
    assert!(result.is_err(), "second check-in should have failed");
}

#[tokio::test]
async fn sold_out_event_rejects_extra_tickets() {
    let program_id = Pubkey::new_unique();
    let (mut banks_client, payer, recent_blockhash) = program_test(program_id).start().await;

    let organizer = Keypair::new();
    let buyer = Keypair::new();
    for kp in [&organizer, &buyer] {
        let tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(&payer.pubkey(), &kp.pubkey(), 10_000_000_000)],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        banks_client.process_transaction(tx).await.unwrap();
    }

    // Event with max_tickets = 1.
    let (init_ix, event_pda) = initialize_event_ix(program_id, organizer.pubkey(), 7, 1, 500_000);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&organizer.pubkey()),
        &[&organizer],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    // First ticket succeeds.
    let mint1 = Keypair::new();
    let buyer_ata1 = get_associated_token_address(&buyer.pubkey(), &mint1.pubkey());
    let create_ata_ix1 = create_associated_token_account(
        &buyer.pubkey(),
        &buyer.pubkey(),
        &mint1.pubkey(),
        &spl_token::id(),
    );
    let (issue_ix1, _ticket_pda1) =
        issue_ticket_ix(program_id, buyer.pubkey(), event_pda, mint1.pubkey(), buyer_ata1);
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix1, issue_ix1],
        Some(&buyer.pubkey()),
        &[&buyer, &mint1],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    // Second ticket must fail: event is sold out.
    let mint2 = Keypair::new();
    let buyer_ata2 = get_associated_token_address(&buyer.pubkey(), &mint2.pubkey());
    let create_ata_ix2 = create_associated_token_account(
        &buyer.pubkey(),
        &buyer.pubkey(),
        &mint2.pubkey(),
        &spl_token::id(),
    );
    let (issue_ix2, _ticket_pda2) =
        issue_ticket_ix(program_id, buyer.pubkey(), event_pda, mint2.pubkey(), buyer_ata2);
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix2, issue_ix2],
        Some(&buyer.pubkey()),
        &[&buyer, &mint2],
        recent_blockhash,
    );
    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "issuing beyond max_tickets should fail");
}

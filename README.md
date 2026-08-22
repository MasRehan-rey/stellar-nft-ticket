nft-ticket (Soroban / Stellar)
Kontrak Soroban untuk sistem tiket event. Ini versi ulang dari kontrak
sebelumnya yang tadinya ditulis untuk Solana — arsitekturnya beda total,
karena Soroban tidak punya konsep SPL Token, PDA, atau CPI seperti Solana.
Kenapa didesain begini
Di Soroban, cara paling sederhana & idiomatik untuk merepresentasikan
"1 item unik milik 1 orang" (mirip NFT) adalah lewat storage kontrak
sendiri — bukan lewat kontrak token terpisah. Jadi tiket di sini bukan
"token yang di-mint", tapi baris data di storage kontrak: `ticket_id -> {event_id, owner, used}`.
Konsep	Implementasi di Soroban
"1 tiket"	Entry storage dengan key `Ticket(ticket_id: u64)`
Kepemilikan	Field `owner: Address` di dalam `Ticket`
Otorisasi	`require_auth()` — hanya pemilik address yang bisa vote/act sebagai dirinya
Anti-oversell	`Event.tickets_issued` dibandingkan ke `Event.max_tickets`
Check-in / redeem	Field `used` di-set `true`, dicek sebelum boleh check-in lagi
Transfer tiket	Fungsi `transfer_ticket` ganti `owner`, ditolak kalau tiket sudah `used`
Struktur data
```rust
pub struct Event {
    pub organizer: Address,
    pub max_tickets: u32,
    pub tickets_issued: u32,
    pub price: i128,       // stroops, informasional
}

pub struct Ticket {
    pub event_id: u64,
    pub owner: Address,
    pub used: bool,
}
```
Fungsi kontrak
`initialize_event(organizer, event_id, max_tickets, price)` — buat event baru. Butuh otorisasi `organizer`.
`issue_ticket(event_id, buyer) -> ticket_id` — mint 1 tiket ke `buyer`. Butuh otorisasi `buyer`. Gagal kalau event sudah sold out.
`check_in_ticket(ticket_id, holder)` — tandai tiket sudah dipakai. Butuh otorisasi `holder`, dan `holder` harus benar-benar pemilik tiket, serta belum pernah check-in.
`transfer_ticket(ticket_id, from, to)` — pindahkan kepemilikan tiket yang belum dipakai. Butuh otorisasi `from`.
`get_event(event_id) -> Event` / `get_ticket(ticket_id) -> Ticket` — baca data (view function).
Kode error
Kode	Arti
1	`EventAlreadyExists`
2	`EventNotFound`
3	`SoldOut`
4	`TicketNotFound`
5	`NotTicketOwner`
6	`AlreadyUsed`
Build & test
Pastikan nama folder & `name` di `Cargo.toml` tidak mengandung spasi
(cuma huruf/angka/`-`/`_`) — ini penyebab error `cargo metadata` yang
sebelumnya kamu alami.
```bash
cd contracts/nft-ticket

# unit test (jalan di luar WASM, cepat untuk iterasi)
cargo test

# build .wasm yang siap deploy
stellar contract build
# kalau masih pakai CLI lama: soroban contract build
```
Hasil build ada di `target/wasm32-unknown-unknown/release/nft_ticket.wasm`.
Deploy & coba manual lewat Stellar CLI
```bash
# deploy ke testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/nft_ticket.wasm \
  --source <akun-kamu> \
  --network testnet

# panggil initialize_event
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <akun-organizer> \
  --network testnet \
  -- initialize_event \
  --organizer <G_ADDRESS_ORGANIZER> \
  --event_id 1 \
  --max_tickets 100 \
  --price 10000000

# beli / issue tiket
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <akun-buyer> \
  --network testnet \
  -- issue_ticket \
  --event_id 1 \
  --buyer <G_ADDRESS_BUYER>
```
Menambahkan ke workspace Soroban Studio
Tambahkan folder ini ke `members` di root `Cargo.toml` workspace:
```toml
[workspace]
members = [
    "contracts/nft-ticket",
]
```
Yang belum ada (kalau mau dikembangkan)
Pembayaran on-chain saat `issue_ticket` — saat ini `price` di `Event`
cuma informasi. Untuk benar-benar menagih, integrasikan dengan Stellar
Asset Contract (native XLM atau custom asset) via cross-contract call
dari `issue_ticket`.
Admin/organizer-only check-in — saat ini siapa pun bisa memanggil
`check_in_ticket` asalkan dia `require_auth()` sebagai `owner` tiket;
kalau mau hanya staff venue yang bisa "memvalidasi" tiket (bukan
pemegangnya sendiri), tambahkan field `validator: Address` di `Event`
dan cek otorisasi itu di `check_in_ticket`.
Metadata event (nama, gambar, lokasi) — bisa ditambah sebagai field
string/bytes di `Event`, atau disimpan off-chain dan cukup di-hash on-chain.
Kedaluwarsa tiket/event berdasarkan `env.ledger().timestamp()`.
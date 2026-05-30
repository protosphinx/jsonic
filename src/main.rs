//! Jsonic Protocol Demo
//!
//! Demonstrates the full lifecycle:
//! 1. Register DAOs (businesses) on the network
//! 2. DAOs exchange invoices and payments
//! 3. POT validates and matches transactions
//! 4. Side-chains record ledger entries
//! 5. Solstice syncs to main-chain and mints tokens

use jsonic_protocol::core::dao::RegisteredDAO;
use jsonic_protocol::core::heartbeat::JsonicNode;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              JSONIC PROTOCOL — REFERENCE NODE               ║");
    println!("║     B2B Blockchain with Proof of Transaction (POT)          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // -----------------------------------------------------------------------
    // 1. Initialize the Jsonic network node
    // -----------------------------------------------------------------------
    let mut node = JsonicNode::new();
    node.solstice_interval = 10; // Short interval for demo purposes

    println!(
        "▸ Network node initialized (Solstice every {} heartbeats)",
        node.solstice_interval
    );
    println!();

    // -----------------------------------------------------------------------
    // 2. Register DAOs (businesses joining the network)
    // -----------------------------------------------------------------------
    let mut acme = RegisteredDAO::register("Acme Corp", "Technology");
    let mut globex = RegisteredDAO::register("Globex Inc", "Manufacturing");
    let mut initech = RegisteredDAO::register("Initech LLC", "Consulting");

    println!("▸ Registered DAOs:");
    println!(
        "  DAO 1: {} [{}] id={}…",
        acme.dao.profile.name,
        acme.dao.profile.sector,
        &acme.dao.id[..12]
    );
    println!(
        "  DAO 2: {} [{}] id={}…",
        globex.dao.profile.name,
        globex.dao.profile.sector,
        &globex.dao.id[..12]
    );
    println!(
        "  DAO 3: {} [{}] id={}…",
        initech.dao.profile.name,
        initech.dao.profile.sector,
        &initech.dao.id[..12]
    );
    println!();

    let acme_id = acme.id().clone();
    let globex_id = globex.id().clone();
    let initech_id = initech.id().clone();

    node.register_dao(acme.dao.clone());
    node.register_dao(globex.dao.clone());
    node.register_dao(initech.dao.clone());
    println!(
        "▸ Registry indexed {} DAO identities",
        node.registry.iter().count()
    );
    println!();

    // -----------------------------------------------------------------------
    // 3. Business transactions: invoices and payments
    // -----------------------------------------------------------------------
    println!("▸ Submitting B2B transactions…");
    println!();

    // Acme invoices Globex for tech consulting
    let inv1 = acme.create_invoice(&globex_id, 50_000.0, "USD", "Tech consulting Q1 2024");
    println!(
        "  📄 Invoice: {} → {} | $50,000 | '{}'",
        acme.dao.profile.name, globex.dao.profile.name, inv1.description
    );
    let inv1_id = inv1.id.clone();
    node.submit_transaction(inv1).expect("submit invoice 1");

    // Globex pays Acme's invoice
    let pay1 = globex.create_payment(
        &acme_id,
        50_000.0,
        "USD",
        &inv1_id,
        "Payment for tech consulting Q1",
    );
    println!(
        "  💰 Payment: {} → {} | $50,000 | settles invoice",
        globex.dao.profile.name, acme.dao.profile.name
    );
    node.submit_transaction(pay1).expect("submit payment 1");

    // Initech invoices Acme for consulting
    let inv2 = initech.create_invoice(&acme_id, 30_000.0, "USD", "Strategy consulting Jan 2024");
    println!(
        "  📄 Invoice: {} → {} | $30,000 | '{}'",
        initech.dao.profile.name, acme.dao.profile.name, inv2.description
    );
    let inv2_id = inv2.id.clone();
    node.submit_transaction(inv2).expect("submit invoice 2");

    // Acme pays Initech
    let pay2 = acme.create_payment(
        &initech_id,
        30_000.0,
        "USD",
        &inv2_id,
        "Payment for strategy consulting",
    );
    println!(
        "  💰 Payment: {} → {} | $30,000 | settles invoice",
        acme.dao.profile.name, initech.dao.profile.name
    );
    node.submit_transaction(pay2).expect("submit payment 2");

    // Globex invoices Initech for manufacturing
    let inv3 = globex.create_invoice(&initech_id, 75_000.0, "EUR", "Custom parts manufacturing");
    println!(
        "  📄 Invoice: {} → {} | €75,000 | '{}'",
        globex.dao.profile.name, initech.dao.profile.name, inv3.description
    );
    node.submit_transaction(inv3).expect("submit invoice 3");
    // This invoice is NOT paid — it will remain unmatched

    println!();

    // -----------------------------------------------------------------------
    // 4. Run heartbeats to process transactions and trigger Solstice
    // -----------------------------------------------------------------------
    println!("▸ Running heartbeat loop…");
    println!();

    for tick in 1..=node.solstice_interval {
        match node.heartbeat() {
            Some(distributions) => {
                println!(
                    "  ♥ Heartbeat #{}: ★ SOLSTICE — Main-chain block #{} created",
                    tick,
                    node.main_chain.height()
                );
                println!();

                // Print token distribution
                println!("  ┌─────────────────────────────────────────────────────┐");
                println!("  │              TOKEN DISTRIBUTION                     │");
                println!("  ├─────────────────────────────────────────────────────┤");
                for dist in &distributions {
                    let name = node
                        .registry
                        .get(&dist.dao_id)
                        .map(|d| d.profile.name.as_str())
                        .unwrap_or("Unknown");
                    println!("  │  {} : {:.4} tokens", name, dist.tokens_awarded);
                    println!("  │    └─ {}", dist.reason);
                }
                println!("  └─────────────────────────────────────────────────────┘");
                println!();
            }
            None => {
                print!("  ♥ Heartbeat #{} ", tick);
                if node.pending_count() > 0 {
                    print!("({} pending)", node.pending_count());
                }
                println!();
            }
        }
    }

    // -----------------------------------------------------------------------
    // 5. Final state
    // -----------------------------------------------------------------------
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                     FINAL STATE                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    println!("  Main-chain height: {}", node.main_chain.height());
    println!(
        "  Effective heartbeat: {} ms",
        node.effective_heartbeat_ms()
    );
    println!(
        "  Total token supply: {:.4}",
        node.main_chain.total_token_supply
    );
    println!("  Network anxiety: {:.4}", node.main_chain.anxiety());
    println!();

    for (id, chain) in &node.side_chains {
        let dao = node.registry.get(id).unwrap();
        let balance = chain.current_balance();
        println!("  {} ({}…):", dao.profile.name, &id[..12]);
        println!("    Side-chain height:    {}", chain.height());
        println!(
            "    Accounts receivable:  ${:.2}",
            balance.accounts_receivable
        );
        println!("    Accounts payable:     ${:.2}", balance.accounts_payable);
        println!("    Revenue:              ${:.2}", balance.revenue);
        println!("    Expenses:             ${:.2}", balance.expenses);
        println!("    Net position:         ${:.2}", balance.net_position());
        println!("    Token balance:        {:.4}", dao.token_balance);
        println!();
    }
}

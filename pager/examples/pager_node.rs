#!/usr/bin/env rust
//! Pager Node Example
//!
//! Starts a pager process on a cluster node that:
//! 1. Initializes transport (TCP by default, RDMA if available)
//! 2. Registers transport endpoint with coordinator
//! 3. Discovers peer endpoints
//! 4. Listens for remote page requests
//!
//! Usage: pager_node <node_id> <total_nodes> <coordinator_url>
//!
//! Example: pager_node 0 2 http://100.119.10.82:8000

use pager::start_pager;
use std::env;
use std::io::Write;
use std::process;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <node_id> <total_nodes> <coordinator_url>",
            args[0]
        );
        eprintln!("Example: {} 0 2 http://100.119.10.82:8000", args[0]);
        process::exit(1);
    }

    let node_id: u32 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Error: node_id must be a number");
        process::exit(1);
    });

    let total_nodes: u32 = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Error: total_nodes must be a number");
        process::exit(1);
    });

    let coordinator_url = &args[3];

    println!("🚀 Starting Pager Node");
    println!("======================");
    println!("Node ID: {}", node_id);
    println!("Total Nodes: {}", total_nodes);
    println!("Coordinator: {}", coordinator_url);
    println!();

    // Allocate 1GB of memory for this pager
    let memory_size = 1024 * 1024 * 1024; // 1GB

    // Allocate the memory region
    let base_ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            memory_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if base_ptr == libc::MAP_FAILED {
        eprintln!("❌ Failed to allocate memory");
        process::exit(1);
    }

    println!(
        "✓ Allocated {}MB memory at {:p}",
        memory_size / (1024 * 1024),
        base_ptr
    );

    // Start the pager
    println!("✓ Starting pager with transport...");

    match start_pager(
        base_ptr as *mut u8,
        memory_size,
        node_id,
        total_nodes,
        coordinator_url,
    ) {
        Ok(handle) => {
            println!("✅ Pager started successfully!");
            println!();
            println!("📊 Status:");
            println!("   - Transport initialized and endpoint registered");
            println!("   - Listening for remote page requests");
            println!("   - Ready to serve pages to peers");
            println!();
            println!("Press Ctrl+C to stop...");
            println!();

            // Keep the process running
            let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let r = running.clone();

            // Set up Ctrl+C handler
            ctrlc::set_handler(move || {
                println!("\n🛑 Shutting down pager...");
                r.store(false, std::sync::atomic::Ordering::SeqCst);
            })
            .expect("Error setting Ctrl+C handler");

            // Main loop - just keep process alive
            let mut counter = 0;
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(5));
                counter += 5;

                // Print status every 30 seconds
                if counter % 30 == 0 {
                    print!(".");
                    std::io::stdout().flush().unwrap();
                }
            }

            println!("\n✅ Pager stopped cleanly");

            // Clean up memory
            unsafe {
                libc::munmap(base_ptr, memory_size);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to start pager: {}", e);

            // Clean up memory
            unsafe {
                libc::munmap(base_ptr, memory_size);
            }

            process::exit(1);
        }
    }
}

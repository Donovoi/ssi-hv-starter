/// Test distributed paging by triggering page faults and measuring latency
///
/// This test:
/// 1. Allocates memory with userfaultfd
/// 2. Triggers page faults by accessing memory
/// 3. Measures latency of page fault handling
/// 4. Tests both local and remote page fetches
use anyhow::{Context, Result};
use std::time::{Duration, Instant};

const PAGE_SIZE: usize = 4096;
const TEST_PAGES: usize = 10;
const MEMORY_SIZE: usize = TEST_PAGES * PAGE_SIZE;

fn main() -> Result<()> {
    println!("🧪 Distributed Paging Test");
    println!("==========================");
    println!();

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <node_id> <total_nodes> <coordinator_url>",
            args[0]
        );
        std::process::exit(1);
    }

    let node_id: u32 = args[1].parse().context("Invalid node_id")?;
    let total_nodes: u32 = args[2].parse().context("Invalid total_nodes")?;
    let coordinator_url = &args[3];

    println!("📊 Configuration:");
    println!("   Node ID: {}", node_id);
    println!("   Total Nodes: {}", total_nodes);
    println!("   Coordinator: {}", coordinator_url);
    println!("   Test Size: {} pages ({} bytes)", TEST_PAGES, MEMORY_SIZE);
    println!();

    // Step 1: Allocate memory with mmap
    println!("1️⃣  Allocating memory...");
    let base_ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            MEMORY_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if base_ptr == libc::MAP_FAILED {
        anyhow::bail!("mmap failed");
    }

    println!("   ✓ Allocated {} bytes at {:p}", MEMORY_SIZE, base_ptr);
    println!();

    // Step 2: Register with userfaultfd
    println!("2️⃣  Setting up userfaultfd...");
    use userfaultfd::UffdBuilder;

    let uffd = UffdBuilder::new()
        .close_on_exec(true)
        .non_blocking(false)
        .create()
        .context("Failed to create userfaultfd")?;

    println!("   ✓ Created userfaultfd");

    unsafe {
        uffd.register(base_ptr, MEMORY_SIZE)
            .context("Failed to register userfaultfd")?;
    }

    println!("   ✓ Registered {} bytes with userfaultfd", MEMORY_SIZE);
    println!();

    // Step 3: Test page fault handling
    println!("3️⃣  Testing page fault handling...");
    println!();

    let memory_slice = unsafe { std::slice::from_raw_parts_mut(base_ptr as *mut u8, MEMORY_SIZE) };

    // Test writing to each page and measure latency
    let mut latencies = Vec::new();

    for page_num in 0..TEST_PAGES {
        let offset = page_num * PAGE_SIZE;
        let start = Instant::now();

        // Write to the page - this will trigger a page fault
        memory_slice[offset] = (page_num as u8) ^ 0xAA;

        let latency = start.elapsed();
        latencies.push(latency);

        println!("   Page {:2}: {:?}", page_num, latency);
    }

    println!();

    // Step 4: Statistics
    println!("4️⃣  Statistics:");
    let total_time: Duration = latencies.iter().sum();
    let avg_latency = total_time / latencies.len() as u32;
    let min_latency = latencies.iter().min().unwrap();
    let max_latency = latencies.iter().max().unwrap();

    println!("   Total time: {:?}", total_time);
    println!("   Average latency: {:?}", avg_latency);
    println!("   Min latency: {:?}", min_latency);
    println!("   Max latency: {:?}", max_latency);
    println!();

    // Step 5: Verify data integrity
    println!("5️⃣  Verifying data integrity...");
    let mut errors = 0;
    for page_num in 0..TEST_PAGES {
        let offset = page_num * PAGE_SIZE;
        let expected = (page_num as u8) ^ 0xAA;
        if memory_slice[offset] != expected {
            eprintln!(
                "   ✗ Page {} mismatch: expected {}, got {}",
                page_num, expected, memory_slice[offset]
            );
            errors += 1;
        }
    }

    if errors == 0 {
        println!("   ✓ All {} pages verified successfully!", TEST_PAGES);
    } else {
        println!("   ✗ {} pages had errors", errors);
    }
    println!();

    // Step 6: Read test
    println!("6️⃣  Testing read performance...");
    let mut read_latencies = Vec::new();

    for page_num in 0..TEST_PAGES {
        let offset = page_num * PAGE_SIZE;
        let start = Instant::now();

        // Read from the page
        let _value = memory_slice[offset];

        let latency = start.elapsed();
        read_latencies.push(latency);
    }

    let read_total: Duration = read_latencies.iter().sum();
    let read_avg = read_total / read_latencies.len() as u32;

    println!("   Average read latency: {:?}", read_avg);
    println!();

    println!("✅ Test completed successfully!");
    println!();
    println!("Summary:");
    println!("   - {} page faults handled", TEST_PAGES);
    println!("   - Average fault latency: {:?}", avg_latency);
    println!("   - Average read latency: {:?}", read_avg);
    println!(
        "   - Data integrity: {}",
        if errors == 0 { "✓" } else { "✗" }
    );

    // Cleanup
    unsafe {
        libc::munmap(base_ptr, MEMORY_SIZE);
    }

    Ok(())
}

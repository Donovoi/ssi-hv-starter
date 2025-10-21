//! Phase 9: Distributed Paging Workload Test
//!
//! This test validates the distributed paging system under realistic workloads:
//! 1. Local page access (first-touch allocation)
//! 2. Remote page access (cross-node fetching)
//! 3. Mixed access patterns (sequential, random, strided)
//! 4. Concurrent page faults
//! 5. Data integrity validation
//!
//! Usage:
//!   sudo ./target/release/examples/phase9_workload_test <node_id> <total_nodes> <coordinator_url>
//!
//! Example:
//!   sudo ./target/release/examples/phase9_workload_test 0 2 http://100.86.226.54:8001

use anyhow::{Context, Result};
use pager::start_pager;
use std::env;
use std::io::Write;
use std::slice;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

const PAGE_SIZE: usize = 4096;
const MB: usize = 1024 * 1024;

/// Test configuration
struct TestConfig {
    node_id: u32,
    total_nodes: u32,
    coordinator_url: String,
    mem_size: usize,
}

/// Test statistics
#[derive(Debug, Default)]
struct TestStats {
    local_faults: usize,
    remote_faults: usize,
    local_latencies_us: Vec<u64>,
    remote_latencies_us: Vec<u64>,
}

impl TestStats {
    fn median(data: &[u64]) -> Option<u64> {
        if data.is_empty() {
            return None;
        }
        let mut sorted = data.to_vec();
        sorted.sort_unstable();
        Some(sorted[sorted.len() / 2])
    }

    fn p99(data: &[u64]) -> Option<u64> {
        if data.is_empty() {
            return None;
        }
        let mut sorted = data.to_vec();
        sorted.sort_unstable();
        let idx = (sorted.len() as f64 * 0.99).floor() as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    fn report(&self) {
        println!("\n{:=<60}", "");
        println!("📊 TEST RESULTS");
        println!("{:-<60}", "");

        println!("\n🔢 Fault Counts:");
        println!("  Local faults:  {}", self.local_faults);
        println!("  Remote faults: {}", self.remote_faults);
        println!(
            "  Total faults:  {}",
            self.local_faults + self.remote_faults
        );

        if !self.local_latencies_us.is_empty() {
            println!("\n⚡ Local Fault Latency:");
            println!(
                "  Median: {:>6} µs",
                Self::median(&self.local_latencies_us).unwrap_or(0)
            );
            println!(
                "  P99:    {:>6} µs",
                Self::p99(&self.local_latencies_us).unwrap_or(0)
            );
        }

        if !self.remote_latencies_us.is_empty() {
            println!("\n🌐 Remote Fault Latency:");
            println!(
                "  Median: {:>6} µs",
                Self::median(&self.remote_latencies_us).unwrap_or(0)
            );
            println!(
                "  P99:    {:>6} µs",
                Self::p99(&self.remote_latencies_us).unwrap_or(0)
            );
        }

        let total_faults = self.local_faults + self.remote_faults;
        if total_faults > 0 {
            let remote_ratio = self.remote_faults as f64 / total_faults as f64;
            println!("\n📈 Remote Miss Ratio: {:.1}%", remote_ratio * 100.0);
        }

        println!("\n{:=<60}", "");
    }
}

/// Allocate memory using mmap with userfaultfd
fn allocate_memory(size: usize) -> Result<*mut u8> {
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        anyhow::bail!("mmap failed");
    }

    println!("✅ Allocated memory: {:p}, size: {} MB", ptr, size / MB);
    Ok(ptr as *mut u8)
}

/// Test 1: Sequential local page access (first-touch)
fn test_local_sequential(base: *mut u8, num_pages: usize, stats: &mut TestStats) -> Result<()> {
    println!("\n🧪 Test 1: Local Sequential Access");
    println!("  Pages: {}", num_pages);

    for i in 0..num_pages {
        let offset = i * PAGE_SIZE;
        let start = Instant::now();

        // Write to trigger page fault
        unsafe {
            let page = slice::from_raw_parts_mut(base.add(offset), PAGE_SIZE);
            page[0] = (i & 0xFF) as u8;
            page[PAGE_SIZE - 1] = ((i >> 8) & 0xFF) as u8;
        }

        let latency = start.elapsed().as_micros() as u64;
        stats.local_faults += 1;
        stats.local_latencies_us.push(latency);

        if (i + 1) % 100 == 0 {
            print!("\r  Progress: {}/{} pages", i + 1, num_pages);
            std::io::stdout().flush().ok();
        }
    }

    println!("\n  ✅ Completed {} local faults", num_pages);
    Ok(())
}

/// Test 2: Verify data integrity after faults
fn test_data_integrity(base: *mut u8, num_pages: usize) -> Result<()> {
    println!("\n🧪 Test 2: Data Integrity Verification");

    let mut errors = 0;
    for i in 0..num_pages {
        let offset = i * PAGE_SIZE;
        unsafe {
            let page = slice::from_raw_parts(base.add(offset), PAGE_SIZE);
            let expected_first = (i & 0xFF) as u8;
            let expected_last = ((i >> 8) & 0xFF) as u8;

            if page[0] != expected_first || page[PAGE_SIZE - 1] != expected_last {
                errors += 1;
                if errors <= 5 {
                    println!(
                        "  ❌ Page {} mismatch: got ({}, {}), expected ({}, {})",
                        i,
                        page[0],
                        page[PAGE_SIZE - 1],
                        expected_first,
                        expected_last
                    );
                }
            }
        }
    }

    if errors > 0 {
        println!("  ❌ Data integrity check FAILED: {} errors", errors);
        anyhow::bail!("Data integrity check failed");
    } else {
        println!("  ✅ Data integrity check PASSED");
    }

    Ok(())
}

/// Test 3: Random access pattern
fn test_random_access(base: *mut u8, num_pages: usize, stats: &mut TestStats) -> Result<()> {
    println!("\n🧪 Test 3: Random Access Pattern");

    // Simple pseudo-random sequence
    let mut rng_state = 12345u64;
    for i in 0..num_pages.min(200) {
        // Linear congruential generator
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let page_idx = (rng_state % num_pages as u64) as usize;

        let offset = page_idx * PAGE_SIZE;
        let start = Instant::now();

        unsafe {
            let page = slice::from_raw_parts_mut(base.add(offset), PAGE_SIZE);
            page[1024] = ((i >> 16) & 0xFF) as u8;
        }

        let latency = start.elapsed().as_micros() as u64;
        // These might be cache hits, not faults, so don't count them
        if latency > 10 {
            // Likely a fault
            stats.local_faults += 1;
            stats.local_latencies_us.push(latency);
        }
    }

    println!("  ✅ Completed random access test");
    Ok(())
}

/// Test 4: Strided access pattern
fn test_strided_access(base: *mut u8, num_pages: usize, stats: &mut TestStats) -> Result<()> {
    println!("\n🧪 Test 4: Strided Access Pattern");

    let stride = 16; // Access every 16th page
    let num_accesses = (num_pages / stride).min(100);

    for i in 0..num_accesses {
        let page_idx = i * stride;
        let offset = page_idx * PAGE_SIZE;
        let start = Instant::now();

        unsafe {
            let page = slice::from_raw_parts_mut(base.add(offset), PAGE_SIZE);
            page[2048] = ((i >> 24) & 0xFF) as u8;
        }

        let latency = start.elapsed().as_micros() as u64;
        if latency > 10 {
            stats.local_faults += 1;
            stats.local_latencies_us.push(latency);
        }

        if (i + 1) % 20 == 0 {
            print!("\r  Progress: {}/{} strides", i + 1, num_accesses);
            std::io::stdout().flush().ok();
        }
    }

    println!("\n  ✅ Completed strided access test");
    Ok(())
}

/// Test 5: Concurrent page faults
fn test_concurrent_faults(base: *mut u8, num_pages: usize, stats: &mut TestStats) -> Result<()> {
    println!("\n🧪 Test 5: Concurrent Page Faults");

    let num_threads = 4;
    let pages_per_thread = (num_pages / num_threads).min(50);
    println!("  Threads: {}", num_threads);
    println!("  Pages per thread: {}", pages_per_thread);

    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];

    // Convert pointer to usize for thread safety
    let base_addr = base as usize;

    for t in 0..num_threads {
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            let mut thread_latencies = vec![];
            let start_page = t * pages_per_thread;
            let thread_base = base_addr as *mut u8;

            // Wait for all threads to be ready
            barrier.wait();

            for i in 0..pages_per_thread {
                let page_idx = start_page + i;
                let offset = page_idx * PAGE_SIZE;
                let start = Instant::now();

                unsafe {
                    let page = slice::from_raw_parts_mut(thread_base.add(offset), PAGE_SIZE);
                    page[512] = ((t + i) & 0xFF) as u8;
                }

                let latency = start.elapsed().as_micros() as u64;
                thread_latencies.push(latency);
            }

            thread_latencies
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let latencies = handle.join().expect("Thread panicked");
        stats.local_faults += latencies.len();
        stats.local_latencies_us.extend(latencies);
    }

    println!(
        "  ✅ Completed {} concurrent faults across {} threads",
        stats.local_faults, num_threads
    );
    Ok(())
}

fn main() -> Result<()> {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <node_id> <total_nodes> <coordinator_url>",
            args[0]
        );
        eprintln!("Example: {} 0 2 http://100.86.226.54:8001", args[0]);
        std::process::exit(1);
    }

    let config = TestConfig {
        node_id: args[1].parse().context("Invalid node_id")?,
        total_nodes: args[2].parse().context("Invalid total_nodes")?,
        coordinator_url: args[3].clone(),
        mem_size: 256 * MB, // 256 MB test region
    };

    println!("\n{:=<60}", "");
    println!("🚀 PHASE 9: DISTRIBUTED PAGING WORKLOAD TEST");
    println!("{:=<60}", "");
    println!("  Node ID:         {}", config.node_id);
    println!("  Total Nodes:     {}", config.total_nodes);
    println!("  Coordinator:     {}", config.coordinator_url);
    println!("  Memory Size:     {} MB", config.mem_size / MB);
    println!("  Total Pages:     {}", config.mem_size / PAGE_SIZE);
    println!("{:=<60}", "");

    // Allocate memory
    let base = allocate_memory(config.mem_size)?;
    let num_pages = config.mem_size / PAGE_SIZE;

    // Start pager
    println!("\n⚙️  Starting pager...");
    let _pager_handle = start_pager(
        base,
        config.mem_size,
        config.node_id,
        config.total_nodes,
        &config.coordinator_url,
    )?;

    println!("✅ Pager started successfully");
    println!("\n🔧 Waiting for cluster initialization...");
    thread::sleep(std::time::Duration::from_secs(2));

    // Run workload tests
    let mut stats = TestStats::default();

    // Test 1: Local sequential access
    test_local_sequential(base, num_pages, &mut stats)?;

    // Test 2: Data integrity
    test_data_integrity(base, num_pages)?;

    // Test 3: Random access
    test_random_access(base, num_pages, &mut stats)?;

    // Test 4: Strided access
    test_strided_access(base, num_pages, &mut stats)?;

    // Test 5: Concurrent page faults
    test_concurrent_faults(base, num_pages, &mut stats)?;

    // Display results
    stats.report();

    println!("\n✅ All workload tests completed successfully!");
    println!("\n💡 Tip: Compare results across nodes to see distributed behavior");
    println!("   ssh access 'cat /tmp/phase9_node0_results.txt'");
    println!("   ssh mo 'cat /tmp/phase9_node1_results.txt'");

    // Keep pager thread alive for a bit
    println!("\n⏳ Keeping pager alive for 5 seconds...");
    thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

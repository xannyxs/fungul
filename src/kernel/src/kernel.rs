//! The kernel starts here.
//!
//! Here all the lint rules & additional compile rules will be declared.
//!
//! The program is designed in a Unix-like way under the x86 (i386)
//! architecturethe
//!
//! This is not meant as an actual Kernel, and should not be used in production.

#![no_std] // Don't link to standard library - essential for kernels
#![no_main] // Don't use normal entry points - we define our own

// Testing
#![feature(custom_test_frameworks)]
#![test_runner(crate::tests::test_runner)]
#![reexport_test_harness_main = "test_main"]
// Safety and Documentation
#![feature(strict_provenance_lints)] // Enable stricter pointer safety checks
#![feature(abi_x86_interrupt)]
#![feature(dropck_eyepatch)]
#![feature(linked_list_cursors)]
#![feature(allocator_api)]
#![deny(fuzzy_provenance_casts)] // Enforce proper pointer provenance
#![warn(missing_docs)] // Require documentation for public items
#![deny(unsafe_op_in_unsafe_fn)] // Require explicit unsafe blocks even in unsafe functions
#![deny(rustdoc::broken_intra_doc_links)] // Catch broken documentation links

// Code Quality
#![deny(unreachable_pub)] // Catch unnecessarily public items
#![deny(unused_must_use)] // Enforce handling of Result/Option returns
#![deny(unused_crate_dependencies)] // Catch unused dependencies
#![deny(clippy::unwrap_used)] // Prevent unwrap() in kernel code
#![deny(clippy::expect_used)] // Prevent expect() in kernel code
#![deny(clippy::implicit_return)] // Force return keyword
#![allow(clippy::needless_return)] // Allow return keyword

// Memory Safety
#![deny(invalid_reference_casting)] // Prevent dangerous reference casts

// Style and Consistency
#![allow(clippy::tabs_in_doc_comments)] // Your existing allowance for tabs
#![deny(clippy::implicit_clone)] // Make cloning explicit
#![deny(clippy::needless_pass_by_value)] // Optimize parameter passing

// Development Helpers
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

/* -------------------------------------- */

/// Specific Bare Metal support
pub mod arch;
/// Collectiosn - Datatypes and structures
pub mod collections;
/// Device Support - Keyboard & Mouse
pub mod device;
/// Libc - STD Library (Should move in future)
pub mod libc;
/// Macro directory
pub mod macros;
pub mod memory;
/// Panic
pub mod panic;
pub mod sync;
/// Tests
pub mod tests;
/// TTY Support - Specifically VGA
pub mod tty;

use crate::{arch::x86::multiboot::G_SEGMENTS, sync::Mutex};
use alloc::{boxed::Box, format};
use arch::x86::{
	cpu::halt,
	multiboot::{
		get_biggest_available_segment_index, get_memory_region, MultibootInfo,
		MultibootMmapEntry,
	},
};
use collections::linked_list::Node;
use core::{alloc::Layout, arch::asm, ffi::c_void};
use device::keyboard::Keyboard;
use libc::console::console::Console;
use macros::print;
use memory::{
	allocator::{
		BUDDY_PAGE_ALLOCATOR, EARLY_PHYSICAL_ALLOCATOR, NODE_POOL_ALLOCATOR,
	},
	buddy::BuddyAllocator,
	memblock::{self, MemBlockAllocator},
	node_pool, NodePoolAllocator, PAGE_SIZE,
};
use tty::{
	log::{Logger, StatusProgram},
	serial::SERIAL,
};

extern crate alloc;

/* -------------------------------------- */

const MAGIC_VALUE: u32 = 0x2badb002;

/* extern "C" {
	fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	fn memset(str: *mut c_void, c: i32, len: usize) -> *mut c_void;
	fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
} */

/* -------------------------------------- */

#[no_mangle]
#[doc(hidden)]
pub extern "C" fn kernel_main(
	magic_number: u32,
	boot_info: &'static MultibootInfo,
) -> ! {
	Logger::init("Kernel", Some("Starting initialization"));
	Logger::divider();
	Logger::newline();

	if magic_number != MAGIC_VALUE {
		panic!(
			"Incorrect magic number. Current magic number: 0x{:x}",
			magic_number
		);
	}

	if (boot_info.flags & 0x7) != 0x7 {
		let flags = boot_info.flags;

		panic!(
        "Required flags not set. Expected MBALIGN, MEMINFO, and VIDEO to be set, but flag value is: 0b{:b}",
        flags
    );
	}

	Logger::init(
		"Memory Management",
		Some("Starting memory subsystem initialization"),
	);

	Logger::init_step(
		"Memory Detection",
		"Reading memory map from bootloader",
		true,
	);
	SERIAL.lock().init();

	get_memory_region(boot_info);

	Logger::init_step(
		"Memblock Allocator",
		"Initializing early memory allocator",
		true,
	);

	{
		let mut memblock = EARLY_PHYSICAL_ALLOCATOR.lock();
		memblock.get_or_init(MemBlockAllocator::new);
		match memblock.get_mut() {
    Some(alloc) => {
        alloc.init();
        Logger::ok("Memblock Allocator", Some("Initialization successful"));
    },
    None => panic!(
        "Failed to initialize memory block allocator: unable to retrieve mutable reference. \
        This could indicate that the allocator was not properly initialized or was dropped unexpectedly. \
        Check EARLY_PHYSICAL_ALLOCATOR implementation."
    ),
};
	}

	Logger::init_step(
		"Node Pool Allocator",
		"Initializing Node Pool allocator",
		true,
	);

	{
		let index =
			get_biggest_available_segment_index().expect("No region available");

		let needed_nodes = G_SEGMENTS.lock()[index].size() / PAGE_SIZE;

		println_serial!("{}", G_SEGMENTS.lock()[index].size());

		let pool_layout = Layout::from_size_align(
			needed_nodes * size_of::<Node<usize>>(),
			align_of::<Node<usize>>(),
		)
		.expect("Error while creating a layout");

		let ptr = {
			let mut memblock_guard = EARLY_PHYSICAL_ALLOCATOR.lock();
			let allocator =
				memblock_guard.get_mut().expect("MemBlock not available");
			unsafe { allocator.alloc(pool_layout) }
		};

		if ptr.is_null() {
			panic!("Failed to allocate node pool from MemBlock");
		}

		let pool_base_addr = ptr as usize;
		let node_pool_guard = NODE_POOL_ALLOCATOR.lock();

		node_pool_guard.get_or_init(|| {
			println_serial!(
				"Initializing NodePoolAllocator at {:#x}",
				pool_base_addr
			);

			return NodePoolAllocator::new(pool_base_addr, needed_nodes);
		});

		if node_pool_guard.get().is_none() {
			panic!("NodePoolAllocator failed to initialize.");
		}

		println_serial!("NodePoolAllocator initialized successfully.");
	}

	Logger::init_step(
		"Buddy Allocator",
		"Initializing buddy page allocator",
		true,
	);

	{
		let mut base = 0;
		{
			let guard = EARLY_PHYSICAL_ALLOCATOR.lock();
			let memblock = guard.get().unwrap();

			let regions = memblock.mem_region();
			if regions.is_empty() {
				panic!("Not enough memory space");
			}

			for region in regions.iter() {
				if !region.is_empty() {
					base = region.base();
					break;
				}
			}
		};

		#[allow(clippy::implicit_return)]
		BUDDY_PAGE_ALLOCATOR
			.lock()
			.get_or_init(|| BuddyAllocator::new(base));
	}

	{
		EARLY_PHYSICAL_ALLOCATOR.lock().take();

		if EARLY_PHYSICAL_ALLOCATOR.lock().get().is_some() {
			panic!("EARLY_PHYSICAL_ALLOCATOR (memblock) has not been decommissioned.");
		}
	}

	Logger::divider();
	Logger::status("Memory Management", &StatusProgram::OK);

	/* let test = Box::new("Hallo wereld");
	println_serial!("{}", test);
	let another_test = Box::new("cool");
	println_serial!("{}", test);
	println_serial!("{}", another_test); */

	let mut keyboard = Keyboard::default();
	let mut console = Console::default();

	#[cfg(test)]
	test_main();

	loop {
		let c = match keyboard.input() {
			Some(key) => key,
			None => continue,
		};

		console.add_buffer(c);
	}
}

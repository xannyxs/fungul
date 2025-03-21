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
/// Device Support - Keyboard & Mouse
pub mod device;
/// Libc - STD Library (Should move in future)
pub mod libc;
/// Macro directory
pub mod macros;
/// Memory allocation
//pub mod memory;
/// Panic
pub mod panic;
/// Tests
pub mod tests;
/// TTY Support - Specifically VGA
pub mod tty;

use arch::x86::pic::pic_init;
use core::arch::asm;
use device::keyboard::Keyboard;
use libc::console::{bin::idt::print_idt, console::Console};
use tty::serial::SERIAL;

/* -------------------------------------- */

/// The kernel's name.
pub const NAME: &str = env!("CARGO_PKG_NAME");
/// Current kernel version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/* -------------------------------------- */

const PIC_1_OFFSET: u8 = 20;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[no_mangle]
#[doc(hidden)]
pub extern "C" fn kernel_main() -> ! {
	pic_init(PIC_1_OFFSET, PIC_2_OFFSET);

	unsafe {
		asm!("sti");
		asm!("int $6");
	}

	let mut keyboard = Keyboard::default();
	let mut console = Console::default();
	SERIAL.lock().init();

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

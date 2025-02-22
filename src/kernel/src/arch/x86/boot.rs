//! Boot Configuration (boot.rs)
//!
//! This module handles the Global Descriptor Table (GDT) initialization during
//! the kernel's early boot process. The boot sequence follows these steps:
//!
//! 1. boot.asm: Initial assembly entry point
//! 2. boot.rs:  GDT structure definition and initialization
//! 3. gdt.asm:  CPU flush of GDT entries using LGDT instruction
//!
//! The GDT configuration uses a flat memory model with 4GB segments:
//! - Null Descriptor: Required by CPU architecture (index 0)
//! - Kernel Segments: Ring 0 code and data (indices 1-2)
//! - User Segments:   Ring 3 code and data (indices 3-4)
//!
//! Segment Flags Overview:
//! - Code (0b???1010): Execute/Read
//! - Data (0b???0010): Read/Write
//! - Ring 0 (0b10??): Kernel privilege
//! - Ring 3 (0b11??): User privilege
//! - Size bit (0b??1?): 32-bit protected mode
//! - Granularity (0b???1): 4KB pages

use super::{
	diagnostics::cpu::check_protection_status,
	gdt::{GDTDescriptor, Gate},
};

const PHYSICAL_GDT_ADDRESS: u32 = 0x00000800;
extern "C" {
	// src/arch/{target}/gdt.asm
	fn gdt_flush(gdt_ptr: *const GDTDescriptor);
}

#[doc(hidden)]
pub type GdtGates = [Gate; 5];

/// Global Descriptor Table (GDT) entries that define memory segments and
/// privilege levels. Each entry consists of a base address, size limit, and
/// access permissions.
///
/// The 5 entries are:
/// - [0] Null Descriptor: Required by CPU, must be zero
/// - [1] Kernel Code (Ring 0): Executable segment for kernel code
/// - [2] Kernel Data (Ring 0): Read/write segment for kernel data
/// - [3] User Code (Ring 3): Executable segment for user programs
/// - [4] User Data (Ring 3): Read/write segment for user data
///
/// Access bytes control permissions:
/// - 0b10011010: Ring 0 code (kernel, executable)
/// - 0b10010010: Ring 0 data (kernel, writable)
/// - 0b11111010: Ring 3 code (user, executable)
/// - 0b11110010: Ring 3 data (user, writable)
#[no_mangle]
#[link_section = ".gdt"]
pub static GDT_ENTRIES: GdtGates = [
	Gate(0), // [0] Null Descriptor (CPU requirement)
	#[cfg(target_arch = "x86")]
	Gate::new(0, !0, 0b10011010, 0b1100), // [1] Kernel Code: Ring 0, executable
	Gate::new(0, !0, 0b10010010, 0b1100), // [2] Kernel Data: Ring 0, writable
	Gate::new(0, !0, 0b11111010, 0b1100), // [3] User Code: Ring 3, executable
	Gate::new(0, !0, 0b11110010, 0b1100), // [4] User Data: Ring 3, writable
];

// Future expansion:
// - TSS (Task State Segment) entries will be needed for task switching
// gdt::Gate(0),  // TSS 1
// gdt::Gate(0),  // TSS 2

#[no_mangle]
#[doc(hidden)]
pub fn gdt_init() {
	use core::mem::size_of;

	let gdt_descriptor = GDTDescriptor {
		size: (size_of::<GdtGates>() - 1) as u16,
		offset: PHYSICAL_GDT_ADDRESS,
	};

	unsafe {
		gdt_flush(&gdt_descriptor as *const _);
	}

	check_protection_status();
}

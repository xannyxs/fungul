use crate::memory::RegionType;

#[allow(missing_docs)]
#[cfg(target_arch = "x86")]
#[repr(C, packed)]
pub struct MultibootMmapEntry {
	pub size: u32,
	pub addr: u64,
	pub len: u64,
	pub entry_type: RegionType,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
struct MultibootMmapEntry {
	size: u32,
	addr: u64,
	len: u64,
	entry_type: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct MultibootAoutSymbolTable {
	tabsize: u32,
	strsize: u32,
	addr: u32,
	reserved: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct MultibootElfSection {
	num: u32,
	size: u32,
	addr: u32,
	shndx: u32,
}

/// Represents the Multiboot information structure passed by the bootloader to
/// the kernel. This structure contains various pieces of information about the
/// system and boot process.
#[repr(C, packed)]
pub struct MultibootInfo {
	/// Multiboot information version number and available fields indicator.
	/// Each bit indicates the validity of a particular field in this
	/// structure.
	pub flags: u32,

	/// Amount of lower memory in kilobytes (memory below 1MB).
	/// Only valid if flags[0] is set.
	mem_lower: u32,

	/// Amount of upper memory in kilobytes (memory above 1MB).
	/// Only valid if flags[0] is set.
	mem_upper: u32,

	/// BIOS disk device that the kernel was loaded from.
	/// Only valid if flags[1] is set.
	boot_device: u32,

	/// Physical address of the command line passed to the kernel.
	/// Only valid if flags[2] is set.
	cmdline: u32,

	/// Number of modules loaded along with the kernel.
	/// Only valid if flags[3] is set.
	mods_count: u32,

	/// Physical address of the first module structure.
	/// Only valid if flags[3] is set.
	mods_addr: u32,

	/// Symbol table information for ELF or a.out formats.
	/// Format depends on flags[4] and flags[5].
	syms: [u8; 16],

	/// Length of the memory map buffer provided by the bootloader.
	/// Only valid if flags[6] is set.
	pub mmap_length: u32,

	/// Physical address of the memory map buffer.
	/// Only valid if flags[6] is set.
	pub mmap_addr: u32,

	/// Length of the drives structure.
	/// Only valid if flags[7] is set.
	drives_length: u32,

	/// Physical address of the drives structure.
	/// Only valid if flags[7] is set.
	drives_addr: u32,

	/// Address of ROM configuration table.
	/// Only valid if flags[8] is set.
	config_table: u32,

	/// Physical address of the bootloader's name string.
	/// Only valid if flags[9] is set.
	pub boot_loader_name: *const u8,

	/// Address of APM (Advanced Power Management) table.
	/// Only valid if flags[10] is set.
	apm_table: u32,
}

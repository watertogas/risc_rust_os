use core::fmt;
use core::fmt::Formatter;
use core::fmt::Debug;

use crate::config::KERNEL_PAGE_WIDTH_BITS;
use crate::config::KERNEL_PAGE_SIZE;

const SV39_PA_WIDTH_BITS : usize = 56;
const SV39_PA_MASKS : usize = (1 << SV39_PA_WIDTH_BITS) - 1;
const SV39_VA_WIDTH_BITS : usize = 39;
const SV39_VA_MASKS : usize = (1 << SV39_VA_WIDTH_BITS) - 1;
const SV39_MMU_TABLE_LEVELS : usize = 3;
const SV39_VA_PN_WIDTH_BITS : usize = (SV39_VA_WIDTH_BITS - KERNEL_PAGE_WIDTH_BITS) / SV39_MMU_TABLE_LEVELS;
const SV39_VA_PN_MASK : usize = (1 << SV39_VA_PN_WIDTH_BITS) - 1;
const SV39_PPN_WIDTH_BITS : usize = SV39_PA_WIDTH_BITS - KERNEL_PAGE_WIDTH_BITS;
pub const SV39_PPN_MASKS : usize = (1 << SV39_PPN_WIDTH_BITS) - 1;
const SV39_VPN_WIDTH_BITS : usize = SV39_VA_WIDTH_BITS - KERNEL_PAGE_WIDTH_BITS;
const SV39_VPN_MASKS : usize = (1 << SV39_VPN_WIDTH_BITS) - 1;
pub const USIZE_MAX : usize = 0xFFFFFFFFFFFFFFFF;
pub const ALIGN_4K_MASK : usize = USIZE_MAX << KERNEL_PAGE_WIDTH_BITS;
pub const ALIGN_4K_LOWER_MASK : usize = (1 << KERNEL_PAGE_WIDTH_BITS) - 1;
pub const ALIGN_2M_MASK : usize = USIZE_MAX << (KERNEL_PAGE_WIDTH_BITS + SV39_VA_PN_WIDTH_BITS);
pub const ALIGN_2M_LOWER_MASK : usize = (1 << (KERNEL_PAGE_WIDTH_BITS + SV39_VA_PN_WIDTH_BITS)) - 1;
pub const ALIGN_1G_MASK : usize = USIZE_MAX << (KERNEL_PAGE_WIDTH_BITS + SV39_VA_PN_WIDTH_BITS*2);
pub const ALIGN_1G_LOWER_MASK : usize = (1 << (KERNEL_PAGE_WIDTH_BITS + SV39_VA_PN_WIDTH_BITS*2)) - 1;

/// Definitions
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

/// Definitions
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtualAddr(pub usize);

/// Definitions
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// Definitions
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct VirtPageNum(pub usize);

//Debug method when using print
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PhysAddr:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PhysPageNum:{:#x}", self.0))
    }
}
impl Debug for VirtualAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VirtualAddr:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VirtPageNum:{:#x}", self.0))
    }
}

//transforms from usize to types
impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self { Self(v & SV39_PA_MASKS) }
}

impl From<PhysAddr> for usize{
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}

impl From<usize> for VirtualAddr {
    fn from(value: usize) -> Self {
        Self(value & SV39_VA_MASKS)
    }
}

impl From<VirtualAddr> for usize{
    fn from(value: VirtualAddr) -> Self {
        if value.0 >= (1 << (SV39_VA_WIDTH_BITS - 1)) {
            value.0 | (!((1 << SV39_VA_WIDTH_BITS) - 1))
        } else {
            value.0
        }
    }
}

impl From<usize> for PhysPageNum{
    fn from(value: usize) -> Self {
        Self(value & SV39_PPN_MASKS)
    }
}

impl From<PhysPageNum> for usize{
    fn from(value: PhysPageNum) -> Self {
        value.0
    }
}

impl From<usize> for VirtPageNum{
    fn from(value: usize) -> Self {
        Self(value & SV39_VPN_MASKS)
    }
}

impl From<VirtPageNum> for usize{
    fn from(value: VirtPageNum) -> Self {
        value.0
    }
}

//addr round down in 4K
pub fn round_down_in_4k(addr : usize) -> usize {
    addr & ALIGN_4K_MASK
}
pub fn round_up_in_4k(addr : usize) -> usize {
    (addr + ALIGN_4K_LOWER_MASK) & ALIGN_4K_MASK
}
//addr round down in 2M
pub fn round_down_in_2m(addr : usize) -> usize {
    addr & ALIGN_2M_MASK
}
pub fn round_up_in_2m(addr : usize) -> usize {
    (addr + ALIGN_2M_LOWER_MASK) & ALIGN_2M_MASK
}
//addr round down in 1G
pub fn round_down_in_1g(addr : usize) -> usize {
    addr & ALIGN_1G_MASK
}
pub fn round_up_in_1g(addr : usize) -> usize {
    (addr + ALIGN_1G_LOWER_MASK) & ALIGN_1G_MASK
}

impl PhysAddr {
    pub fn round_down_in_4k(&self)->PhysAddr {
        Self(round_down_in_4k(self.0))
    }
    pub fn round_down_in_2m(&self)->PhysAddr {
        Self(round_down_in_2m(self.0))
    }
    pub fn round_down_in_1g(&self)->PhysAddr {
        Self(round_down_in_1g(self.0))
    }
    pub fn round_up_in_4k(&self)->PhysAddr {
        Self(round_up_in_4k(self.0))
    }
    pub fn round_up_in_2m(&self)->PhysAddr {
        Self(round_up_in_2m(self.0))
    }
    pub fn round_up_in_1g(&self)->PhysAddr {
        Self(round_up_in_1g(self.0))
    }
    pub fn page_offset(&self)->usize {
        self.0 & (KERNEL_PAGE_SIZE - 1)
    }
    pub fn is_aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl From<PhysAddr> for PhysPageNum{
    fn from(value: PhysAddr) -> Self {
        Self(value.0 >> KERNEL_PAGE_WIDTH_BITS)
    }
}

impl From<PhysPageNum> for PhysAddr{
    fn from(value: PhysPageNum) -> Self {
        Self(value.0 << KERNEL_PAGE_WIDTH_BITS)
    }
}

impl VirtualAddr {
    pub fn round_down_in_4k(&self)->VirtualAddr {
        Self(round_down_in_4k(self.0))
    }
    pub fn round_down_in_2m(&self)->VirtualAddr {
        Self(round_down_in_2m(self.0))
    }
    pub fn round_down_in_1g(&self)->VirtualAddr {
        Self(round_down_in_1g(self.0))
    }
    pub fn round_up_in_4k(&self)->VirtualAddr {
        Self(round_up_in_4k(self.0))
    }
    pub fn round_up_in_2m(&self)->VirtualAddr {
        Self(round_up_in_2m(self.0))
    }
    pub fn round_up_in_1g(&self)->VirtualAddr {
        Self(round_up_in_1g(self.0))
    }
    pub fn page_offset(&self)->usize {
        self.0 & (KERNEL_PAGE_SIZE - 1)
    }
    pub fn is_aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl From<VirtualAddr> for VirtPageNum{
    fn from(value: VirtualAddr) -> Self {
        Self(value.0 >> KERNEL_PAGE_WIDTH_BITS)
    }
}

impl From<VirtPageNum> for VirtualAddr{
    fn from(value: VirtPageNum) -> Self {
        Self(value.0 << KERNEL_PAGE_WIDTH_BITS)
    }
}

impl VirtPageNum {
    pub fn get_table_indexs(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx : [usize; 3] = [0; 3];
        for i in 0..3 {
            idx[i] = vpn & SV39_VA_PN_MASK;
            vpn = vpn >> SV39_VA_PN_WIDTH_BITS;
        }
        idx
    }
}

///Add value by one
pub trait StepByOne {
    ///Add value by one
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
impl StepByOne for PhysPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
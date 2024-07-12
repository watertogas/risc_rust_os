use bitflags::*;
use crate::mm::address::PhysPageNum;
use crate::mm::address::VirtPageNum;
use crate::mm::address::VirtualAddr;
use crate::mm::address::PhysAddr;
use crate::mm::address::SV39_PPN_MASKS;
use crate::config::KERNEL_PAGE_WIDTH_BITS;
use crate::config::KERNEL_PAGE_SIZE;
use crate::mm::kernel_set::RISV_KERNEL_STACK_ARER_START;
use crate::mm::frame_allocator::FrameWrapper;
use alloc::collections::BTreeMap;
use crate::mm::frame_allocator::frame_alloc;
use crate::task::process::get_current_root_ppn;

///PTE format for SV39
///63      54 53      28 27      19 18      10 9   8 7 6 5 4 3 2 1 0
///+----------+----------+----------+----------+-----+-+-+-+-+-+-+-+-+
///| reserved |  PPN[2]  |  PPN[1]  |  PPN[0]  | RSW |D|A|G|U|X|W|R|V|
///+----------+----------+----------+----------+-----+-+-+-+-+-+-+-+-+
///     10         26          9          9       2   1 1 1 1 1 1 1 1

//FLAGS  bits
const RISV_PTE_FLAGS_BITS : usize = 8;
//Reserved bits
const RISV_PTE_RESERVED_BITS : usize = 2;
const RISV_PTE_PPN_OFFSET_BITS : usize = RISV_PTE_FLAGS_BITS + RISV_PTE_RESERVED_BITS;
//const RISV_PTE_PPN_2_BITS : usize = 26;
//const RISV_PTE_PPN_1_BITS : usize = 9;
const RISV_PTE_PPN_0_BITS : usize = 9;
//const RISV_FIRST_LEVEL_BITS : usize = KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS + RISV_PTE_PPN_1_BITS;
//const RISV_FIRST_LEVEL_MASK : usize = (1 << RISV_FIRST_LEVEL_BITS) - 1;
//pub const RISV_SECOND_LEVEL_BITS: usize = KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS;
//pub const RISV_SECOND_LEVEL_MEM_SIZE : usize = 1 << RISV_SECOND_LEVEL_BITS;
//pub const RISV_SECOND_LEVEL_MASK : usize = RISV_SECOND_LEVEL_MEM_SIZE - 1;
//pub const RISV_SECOND_LEVEL_MASK_USIZE : usize = USIZE_MAX << RISV_SECOND_LEVEL_BITS;
// 1GB entry-> 1 table
//  2MB entry -> 4 tables
//  4KB entry-> 11 tables
pub const RISV_FIRST_TABLE_NUM : usize = 1;
pub const RISV_SECOND_TABLE_NUM : usize = 4;
pub const RISV_THIRD_TABLE_NUM : usize = 11;


static mut TABLE_USAGE : [u8; RISV_THIRD_TABLE_NUM] = [1 ; RISV_THIRD_TABLE_NUM];

bitflags! {
    pub struct RisvPTEFlags : u8{
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub entry : usize,
}

impl PageTableEntry {
    pub fn new(ppn : PhysPageNum, flags : RisvPTEFlags) -> Self {
        PageTableEntry {
            entry : ppn.0 << RISV_PTE_PPN_OFFSET_BITS | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry {
            entry : 0,
        }
    }
    pub fn to_ppn(&self) -> PhysPageNum {
        ((self.entry >> RISV_PTE_PPN_OFFSET_BITS) & SV39_PPN_MASKS).into()
    }
    pub fn to_flags(&self) -> RisvPTEFlags {
        RisvPTEFlags::from_bits(self.entry as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.to_flags() & RisvPTEFlags::V) != RisvPTEFlags::empty()
    }
    pub fn is_readable(&self) -> bool {
        (self.to_flags() & RisvPTEFlags::R) != RisvPTEFlags::empty()
    }
    pub fn is_writeable(&self) -> bool {
        (self.to_flags() & RisvPTEFlags::W) != RisvPTEFlags::empty()
    }
    pub fn is_executable(&self) -> bool {
        (self.to_flags() & RisvPTEFlags::X) != RisvPTEFlags::empty()
    }
}

impl From<usize> for PageTableEntry {
    fn from(value: usize) -> Self {
        Self { entry: value }
    }
}

impl From<PageTableEntry> for usize {
    fn from(value: PageTableEntry) -> Self {
        value.entry
    }
}

///set N level block entry
pub fn set_n_level_entry(table_prt : usize, vpn : VirtPageNum, ppn : PhysPageNum, flags : RisvPTEFlags, level : usize)
{
    let vidx: [usize; 3] = vpn.get_table_indexs();
    let table = unsafe {core::slice::from_raw_parts_mut(table_prt as *mut usize , 512)};
    let cur_entry : PageTableEntry= table[vidx[level]].into();
    if !cur_entry.is_valid() {
        let entry = PageTableEntry::new(ppn, RisvPTEFlags::V | flags);
        table[vidx[level]] = entry.into();
    }
}

//Static Page table, which is aligned with 4K,
//used when we know how much area should be mapped
pub struct StaticPageTable{
    pub table : usize,
}

impl StaticPageTable {
    pub fn check_block_table_aligned(&self)
    {
        let first_table_ptr = self.table;
        if (first_table_ptr % KERNEL_PAGE_SIZE) != 0 {
            panic!("Page table not aligned")
        } else {
            println! ("L1 table address:0x{:x}", first_table_ptr);
        }
        let second_table_ptr = first_table_ptr + 4096;
        if (second_table_ptr % KERNEL_PAGE_SIZE) != 0 {
            panic!("Page table not aligned")
        } else {
            println! ("L2 table address:0x{:x}", second_table_ptr);
        }
        println! ("Table area:0x{:x}--0x{:x}", first_table_ptr, first_table_ptr + 4096*16);
    }
    pub fn clean_kpage_table(&self)
    {
        self.check_block_table_aligned();
        let start = self.table;
        let end : usize = start + 4096 * 16;
        (start as usize ..end as usize).for_each(|a|{
            unsafe{(a as *mut u8).write_volatile(0)}
        })
    }
    fn alloc_l3_table(&self)->usize{
        let mut found = 0xFF;
        for i in 0..RISV_THIRD_TABLE_NUM {
            unsafe {
                if TABLE_USAGE[i] == 1 {
                    TABLE_USAGE[i] = 0;
                    found = i;
                    break;
                }
            }
        }
        if found == 0xFF {
            panic!("No empty page table found..")
        }
        found
    }
    fn set_l1_pointer_entry(&self, v_start : VirtualAddr) -> usize{
        let first_table_ptr = self.table;
        let vpn : VirtPageNum = v_start.into();
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let second_table_ptr = first_table_ptr + 4096 * vidx[2] + 4096;
        //entry pointer to next table
        set_n_level_entry(first_table_ptr, vpn, PhysAddr::from(second_table_ptr).into(), RisvPTEFlags::V, 2);
        second_table_ptr
    }    
    fn set_l2_pointer_entry(&self,v_start : VirtualAddr) -> usize {
        let second_table_ptr = self.set_l1_pointer_entry(v_start);
        //check if L2 entry is valid, alloc table if needed
        let vpn : VirtPageNum = VirtualAddr::from(v_start).into();
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let table = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
        let l2_entry : PageTableEntry = table[vidx[1]].into();
        if l2_entry.is_valid() {
            let p_addr : PhysAddr = l2_entry.to_ppn().into();
            p_addr.0
        } else {
            let l3_table_index = self.alloc_l3_table();
            let l3_table_ptr = self.table + 4096 * (RISV_FIRST_TABLE_NUM + RISV_SECOND_TABLE_NUM + l3_table_index);
            set_n_level_entry(second_table_ptr, vpn, PhysAddr::from(l3_table_ptr).into(), RisvPTEFlags::V, 1);
            l3_table_ptr
        }
    }
    //set a 2MB block entry
    pub fn set_l2_block_entry(&self, v_start : VirtualAddr, v_end: VirtualAddr, p_start : PhysAddr, flags : RisvPTEFlags) {
        if v_end.0 > v_start.0 {
            //try set first table
            let second_table_ptr = self.set_l1_pointer_entry(v_start);
            let index : usize = (v_end.0 - v_start.0) >> (KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS);
            //println! ("L2 block entrys:{} v_start:0x{:x}--v_end:0x{:x}", index, v_start.0, v_end.0);
            for i in 0..index {
                let v_addr : usize = v_start.0 + i * (1 << (KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS));
                let p_addr : usize = p_start.0 + i * (1 << (KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS));
                let vpn : VirtPageNum = VirtualAddr::from(v_addr).into();
                let ppn : PhysPageNum = PhysAddr::from(p_addr).into();
                set_n_level_entry(second_table_ptr, vpn, ppn, flags, 1);
            }
        } else {
            println! ("invalid block input: v_start:0x{:x} vs v_end:0x{:x}", v_start.0, v_end.0);
        }
    }
    //set a 4K entry
    pub fn set_l3_page_entry(&self, v_start : VirtualAddr, v_end: VirtualAddr, p_start : PhysAddr, flags : RisvPTEFlags) {
        if v_end.0 > v_start.0 {
            //try set first table
            let l3_table_ptr = self.set_l2_pointer_entry(v_start);
            let index : usize = (v_end.0 - v_start.0) >> (KERNEL_PAGE_WIDTH_BITS);
            //println! ("L3 block entrys:{} v_start:0x{:x}--v_end:0x{:x}", index, v_start.0, v_end.0);
            for i in 0..index {
                let v_addr : usize = v_start.0 + i * (1 << KERNEL_PAGE_WIDTH_BITS);
                let p_addr : usize = p_start.0 + i * (1 << KERNEL_PAGE_WIDTH_BITS);
                let vpn : VirtPageNum = VirtualAddr::from(v_addr).into();
                let ppn : PhysPageNum = PhysAddr::from(p_addr).into();
                set_n_level_entry(l3_table_ptr, vpn, ppn, flags, 0);
            }
        } else {
            println! ("invalid page input: v_start:0x{:x} vs v_end:0x{:x}", v_start.0, v_end.0);
        }
    }
    //clear page table entrys
    pub fn clear_blocks_area(&self, v_start : VirtualAddr, v_end: VirtualAddr)
    {
        if v_end.0 > v_start.0 {
            let first_table_ptr = self.table;
            let vpn : VirtPageNum = v_start.into();
            let vidx: [usize; 3] = vpn.get_table_indexs();
            let second_table_ptr = first_table_ptr + 4096 * vidx[2] + 4096;
            let index : usize = (v_end.0 - v_start.0) >> (KERNEL_PAGE_WIDTH_BITS + RISV_PTE_PPN_0_BITS);
            let second_table = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
            let empty_entry : PageTableEntry = PageTableEntry::empty();
            for i in 0..index {
                second_table[vidx[1] + i] = empty_entry.into();
            }
        } else {
            println! ("invalid block input: v_start:0x{:x} vs v_end:0x{:x}", v_start.0, v_end.0);
        }
    }
    
    pub fn clear_pages_area(&self, v_start : VirtualAddr, v_end: VirtualAddr)
    {
        if v_end.0 > v_start.0 {
            let first_table_ptr = self.table;
            let vpn : VirtPageNum = v_start.into();
            let vidx: [usize; 3] = vpn.get_table_indexs();
            let second_table_ptr = first_table_ptr + 4096 * vidx[2] + 4096;
            let second_table = unsafe {core::slice::from_raw_parts(second_table_ptr as *mut usize , 512)};
            let second_entry : PageTableEntry = second_table[vidx[1]].into();
            let third_table_ppn : PhysPageNum = second_entry.to_ppn();
            let third_table_ptr : usize = PhysAddr::from(third_table_ppn).into();
            let third_table = unsafe {core::slice::from_raw_parts_mut(third_table_ptr as *mut usize , 512)};
            let empty_entry : PageTableEntry = PageTableEntry::empty();
            let index : usize = (v_end.0 - v_start.0) >> (KERNEL_PAGE_WIDTH_BITS);
            for i in 0..index {
                third_table[vidx[0] + i] = empty_entry.into();
            }
        } else {
            println! ("invalid pages input: v_start:0x{:x} vs v_end:0x{:x}", v_start.0, v_end.0);
        }
    }
}

//Dynamic page table, used when we did not know
//how much memory we needed, frame aligned in 4K
pub struct DynamicPageTable{
    //user root pgt, full usr space is supported
    pub root_ppn : PhysPageNum,
    pub page_table : BTreeMap<PhysPageNum, FrameWrapper>,
}
impl DynamicPageTable {
    pub fn new()-> Self {
        Self {
            root_ppn : 0.into(),
            page_table : BTreeMap::new(),
        }
    }
    pub fn set_root_ppn(&mut self, phy_addr : usize) {
        self.root_ppn = PhysAddr::from(phy_addr).into();
    }
    pub fn get_root_stap(&self) -> usize {
        let ppn : usize = self.root_ppn.into();
        (8usize << 60) | ppn
    }
    pub fn check_l2_pointers(&mut self, vpn : VirtPageNum) -> PhysPageNum{
        // check root page
        if self.root_ppn.0 == 0 {
            let frame = frame_alloc().unwrap();
            frame.clear_frame();
            self.root_ppn = frame.ppn;
            self.page_table.insert(self.root_ppn, frame);
        }

        let first_table_ptr : usize = PhysAddr::from(self.root_ppn).into();
        let first_table = unsafe {core::slice::from_raw_parts_mut(first_table_ptr as *mut usize , 512)};
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let mut first_entry: PageTableEntry = first_table[vidx[2]].into();
        if !first_entry.is_valid() {
            //alloc second frame
            let second_frame = frame_alloc().unwrap();
            second_frame.clear_frame();
            let entry = PageTableEntry::new(second_frame.ppn, RisvPTEFlags::V);
            first_table[vidx[2]] = entry.into();
            first_entry = entry;
            self.page_table.insert(second_frame.ppn, second_frame);
        }
        let second_table_ptr : usize = PhysAddr::from(first_entry.to_ppn()).into();
        let second_table = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
        let second_entry : PageTableEntry = second_table[vidx[1]].into();
        if !second_entry.is_valid() {
            //alloc second frame
            let third_frame = frame_alloc().unwrap();
            third_frame.clear_frame();
            let entry = PageTableEntry::new(third_frame.ppn, RisvPTEFlags::V);
            second_table[vidx[1]] = entry.into();
            self.page_table.insert(third_frame.ppn, third_frame);
        }
        let find_entry : PageTableEntry = second_table[vidx[1]].into();
        find_entry.to_ppn()
    }
    //set a l3 pages
    pub fn set_l3_pages(&self, vpn : VirtPageNum, ppn : PhysPageNum, flags : RisvPTEFlags, l3_table_ppn : PhysPageNum) {
        let third_table_ptr : usize = PhysAddr::from(l3_table_ppn).into();
        let third_table = unsafe {core::slice::from_raw_parts_mut(third_table_ptr as *mut usize , 512)};
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let third_entry : PageTableEntry = third_table[vidx[0]].into();
        if !third_entry.is_valid() {
            let entry = PageTableEntry::new(ppn, RisvPTEFlags::V | flags);
            third_table[vidx[0]] = entry.into();
        } else {
            panic!("already exists entry:0x{:x}", third_entry.entry);
        }
    }
    pub fn get_n_level_table_ppn(&self, vpn : VirtPageNum, level : usize)-> PhysPageNum {
        // check root page
        if self.root_ppn.0 == 0 {
            panic!("root ppn not found");
        }
        let first_table_ptr : usize = PhysAddr::from(self.root_ppn).into();
        let first_table = unsafe {core::slice::from_raw_parts_mut(first_table_ptr as *mut usize , 512)};
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let first_entry: PageTableEntry = first_table[vidx[2]].into();
        if !first_entry.is_valid() {
            //alloc second frame
            panic!("l1 table is invalid");
        }else if level == 2 {
            return first_entry.to_ppn();
        }
        let second_table_ptr : usize = PhysAddr::from(first_entry.to_ppn()).into();
        let second_table = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
        let second_entry : PageTableEntry = second_table[vidx[1]].into();
        if !second_entry.is_valid() {
            panic!("l2 table is invalid");
        }
        second_entry.to_ppn()
    }
    //clear page table entrys
    pub fn clear_l2_entrys(&mut self, vpn : VirtPageNum, l2_table_ppn : PhysPageNum)
    {
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let second_table_ptr : usize = PhysAddr::from(l2_table_ppn).into();
        let second_table: &mut [usize] = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
        let entry : PageTableEntry = PageTableEntry::empty();
        let l2_entry : PageTableEntry = second_table[vidx[1]].into();
        second_table[vidx[1]] = entry.into();
        let frame_ppn : PhysPageNum = l2_entry.to_ppn();
        self.page_table.remove(&frame_ppn);
    }
    
    pub fn clear_l3_entrys(&self, vpn : VirtPageNum, l3_table_ppn : PhysPageNum)
    {
        let vidx: [usize; 3] = vpn.get_table_indexs();
        let third_table_ptr : usize = PhysAddr::from(l3_table_ppn).into();
        let third_table: &mut [usize] = unsafe {core::slice::from_raw_parts_mut(third_table_ptr as *mut usize , 512)};
        let entry : PageTableEntry = PageTableEntry::empty();
        third_table[vidx[0]] = entry.into();
    }
    pub fn do_table_walk(&self, vpn : VirtPageNum) -> Option<PhysPageNum> {
        // check root page
        if self.root_ppn.0 == 0 {
            print!("root ppn not found");
            return None;
        }
        do_table_walk_in_4k(self.root_ppn, vpn)
    }
    pub fn add_an_existed_page(&mut self, vpn : VirtPageNum, ppn : PhysPageNum, flags : RisvPTEFlags) {
        let l3_table = self.check_l2_pointers(vpn);
        self.set_l3_pages(vpn, ppn, flags, l3_table);
    }
}

pub fn do_table_walk_in_4k(root_table : PhysPageNum, vpn : VirtPageNum) -> Option<PhysPageNum> {
    let first_table_ptr : usize = PhysAddr::from(root_table).into();
    let first_table = unsafe {core::slice::from_raw_parts_mut(first_table_ptr as *mut usize , 512)};
    let vidx: [usize; 3] = vpn.get_table_indexs();
    let first_entry: PageTableEntry = first_table[vidx[2]].into();
    if !first_entry.is_valid() {
        //alloc second frame
        print!("l1 table is invalid");
        return None;
    }
    let second_table_ptr : usize = PhysAddr::from(first_entry.to_ppn()).into();
    let second_table = unsafe {core::slice::from_raw_parts_mut(second_table_ptr as *mut usize , 512)};
    let second_entry : PageTableEntry = second_table[vidx[1]].into();
    if !second_entry.is_valid() {
        print!("l2 table is invalid");
        return None;
    }
    let third_table_ptr : usize = PhysAddr::from(second_entry.to_ppn()).into();
    let third_table: &mut [usize] = unsafe {core::slice::from_raw_parts_mut(third_table_ptr as *mut usize , 512)};
    let third_entry : PageTableEntry = third_table[vidx[0]].into();
    if !third_entry.is_valid() {
        print!("l3 table is invalid");
        return None;
    }
    Some(third_entry.to_ppn())
}

#[allow(unused)]
pub fn uaddr_to_phy_addr(uaddr : usize) -> usize{
    let table = get_current_root_ppn();
    let start_addr : VirtualAddr = (uaddr).into();
    let start_page : VirtualAddr = start_addr.round_down_in_4k();
    let addr : PhysAddr =  do_table_walk_in_4k(table, start_page.into()).unwrap().into();
    let buf_start = start_addr.0 - start_page.0 + addr.0;
    buf_start
}

pub fn kern_vaddr_to_phy_addr(uaddr : usize) -> usize{
    extern "C" {
        fn kernel_pte_bottom();
    }
    if uaddr >= RISV_KERNEL_STACK_ARER_START{
        let table : PhysAddr = PhysAddr::from(kernel_pte_bottom as usize);
        let start_addr : VirtualAddr = (uaddr).into();
        let start_page : VirtualAddr = start_addr.round_down_in_4k();
        let addr : PhysAddr =  do_table_walk_in_4k(table.into(), start_page.into()).unwrap().into();
        let buf_start = start_addr.0 - start_page.0 + addr.0;
        buf_start
    } else {
        uaddr
    }
}
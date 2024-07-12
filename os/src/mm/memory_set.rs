use core::panic;
use alloc::vec::Vec;
use hashbrown::HashMap;
use bitflags::bitflags;
use crate::mm::address::VirtPageNum;
use crate::mm::address::PhysPageNum;
use crate::mm::address::VirtualAddr;
use crate::mm::address::PhysAddr;
use crate::mm::page_table::RisvPTEFlags;
use crate::mm::page_table::do_table_walk_in_4k;
use crate::task::process::get_current_root_ppn;
use crate::mm::frame_allocator::FrameWrapper;
use crate::mm::address::USIZE_MAX;
use crate::config::KERNEL_PAGE_SIZE;
use crate::config::USER_STACK_SIZE;
//kernel stack area, spare 1GB
pub const MEM_IN_1_GB : usize = 0x40000000;
pub const RISV_TRAP_TEXT_STRAT : usize = USIZE_MAX - KERNEL_PAGE_SIZE + 1;
pub const RISV_TRAP_CONTEXT_END : usize = RISV_TRAP_TEXT_STRAT - MEM_IN_1_GB;
pub const RISV_TRAP_CONTEXT_STRAT : usize = RISV_TRAP_CONTEXT_END - MEM_IN_1_GB;
pub const RISV_USER_STACK_END : usize = RISV_TRAP_CONTEXT_STRAT - MEM_IN_1_GB;
pub const RISV_USER_STACK_START : usize = RISV_USER_STACK_END - MEM_IN_1_GB;
pub const USER_FRAMEBUFFER_MAPPED_ADDR : usize = MEM_IN_1_GB * 128;
use alloc::string::String;
use crate::mm::page_table::DynamicPageTable;
use crate::mm::page_table::StaticPageTable;
use crate::alloc::string::ToString;

use super::address::StepByOne;
use super::frame_allocator::frame_alloc;
use crate::common::memset_usize;

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}
////a static memory area is continuous virtual memory space
#[derive(Copy, Clone)]
pub struct MemoryStaticArea{
    //start of virtual address
    pub vaddr_start : VirtualAddr,
    //start of physical address
    pub vaddr_end : VirtualAddr,
    //area permission
    pub perm : MapPermission
}
//now we use the identical mapping for kernel,this means vaddr == paddr
impl MemoryStaticArea {
    pub fn new(v_start : VirtualAddr,
               v_end : VirtualAddr,
               perms : MapPermission,
                )->Self {
        Self { vaddr_start: v_start, vaddr_end: v_end, perm: perms }
    }
    #[allow(unused)]
    pub fn print_info(&self) {
        println!("vaddr_range:0x{:x}-0x{:x}, size:0x{:x}, permission:{}", 
                self.vaddr_start.0,self.vaddr_end.0, 
                self.vaddr_end.0 - self.vaddr_start.0, self.perm.bits);
    }
    //will never map a area exceeded 1G in normal mapping
    fn set_mapping_normal(&self, valid : bool, table : &StaticPageTable) {
        let addr_start = self.vaddr_start.round_down_in_4k();
        let addr_end =  self.vaddr_end.round_up_in_4k();
        let addr_start_block = self.vaddr_start.round_up_in_2m();
        let addr_end_block = self.vaddr_end.round_down_in_2m();
        let pte_flags = RisvPTEFlags::from_bits(self.perm.bits).unwrap();
        //println!("vpn_addr_start:0x{:x}", addr_start.0);
        //println!("vpn_addr_end:0x{:x}", addr_end.0);
        //println!("vpn_addr_start_block:0x{:x}", addr_start_block.0);
        //println!("vpn_addr_end_block:0x{:x}", addr_end_block.0);
        //we will mapping big page if needed,here vir_addr = phys_addr
        if addr_start_block.0 < addr_end_block.0 {
            if valid {
                table.set_l3_page_entry(addr_start, addr_start_block, addr_start.0.into(), pte_flags);
                table.set_l2_block_entry(addr_start_block, addr_end_block, addr_start_block.0.into(), pte_flags);
                table.set_l3_page_entry(addr_end_block, addr_end, addr_end_block.0.into(), pte_flags);
            } else {
                table.clear_pages_area(addr_start, addr_start_block);
                table.clear_blocks_area(addr_start_block, addr_end_block);
                table.clear_pages_area(addr_end_block, addr_end);
            }
        } else {
            if valid {
                table.set_l3_page_entry(addr_start, addr_end, addr_start.0.into(), pte_flags);
            } else {
                table.clear_pages_area(addr_start, addr_end);
            }
        }
    }
    pub fn map_area_normal(&self, table : &StaticPageTable) {
        self.set_mapping_normal(true, table);
    }
    #[allow(unused)]
    pub fn unmap_area_normal(&self, table : &StaticPageTable) {
        self.set_mapping_normal(false, table);
    }
}

//Memory mapped with frames, which means vaddr != paddr
pub struct FrameBasedArea{
    pub area : MemoryStaticArea,
    //frames used for user
    pub mem_frames : HashMap<VirtPageNum, FrameWrapper>,
}

impl FrameBasedArea {
    pub fn new(in_area : MemoryStaticArea)->Self {
        Self { 
            area : in_area,
            mem_frames : HashMap::new(),
        }
    }
    fn set_l3_frames(&mut self, table : &mut DynamicPageTable, v_start: VirtualAddr, v_end: VirtualAddr, pte_flags : RisvPTEFlags)
    {
        let vpn_start : VirtPageNum = v_start.into();
        let vpn_end : VirtPageNum = v_end.into();
        let l3_table = table.check_l2_pointers(v_start.into());
        if vpn_start.0 < vpn_end.0 {
            let index :  usize = vpn_end.0 - vpn_start.0;
            for i in 0..index {
                let frame : FrameWrapper = frame_alloc().unwrap();
                let cur_vpn : VirtPageNum = VirtPageNum::from(vpn_start.0 + i);
                table.set_l3_pages(cur_vpn, frame.ppn, pte_flags, l3_table);
                self.mem_frames.insert(cur_vpn, frame);
            }
        } else {
            panic!("unexpected area:0x{:x}--0x{:x}", vpn_start.0, vpn_end.0);
        }
    }
    fn set_l2_frames(&mut self, table : &mut DynamicPageTable, v_start: VirtualAddr, v_end: VirtualAddr, pte_flags : RisvPTEFlags){
        let vpn_start : VirtPageNum = v_start.into();
        let vpn_end : VirtPageNum = v_end.into();
        if vpn_start.0 < vpn_end.0 {
            let index :  usize = (vpn_end.0 - vpn_start.0) >> 21;
            let step : usize = 1 << 9;
            for i in 0..index {
                let s_vpn : VirtPageNum = VirtPageNum::from(vpn_start.0 + i*step);
                let e_vpn : VirtPageNum = VirtPageNum::from(vpn_start.0 + i*step + step);
                self.set_l3_frames(table,  s_vpn.into(), e_vpn.into(), pte_flags);
            }
        } else {
            panic!("unexpected area:0x{:x}--0x{:x}", vpn_start.0, vpn_end.0);
        }
    }
    fn free_l3_frames(&mut self, table : &mut DynamicPageTable, v_start: VirtualAddr, v_end: VirtualAddr)
    {
        let vpn_start : VirtPageNum = v_start.into();
        let vpn_end : VirtPageNum = v_end.into();
        let l3_table = table.get_n_level_table_ppn(v_start.into(), 3);
        if vpn_start.0 < vpn_end.0 {
            let index :  usize = vpn_end.0 - vpn_start.0;
            for i in 0..index {
                let cur_vpn : VirtPageNum = VirtPageNum::from(vpn_start.0 + i);
                table.clear_l3_entrys(cur_vpn, l3_table);
                self.mem_frames.remove(&cur_vpn);
            }
        } else {
            panic!("unexpected area:0x{:x}--0x{:x}", vpn_start.0, vpn_end.0);
        }
    }
    fn free_l2_frames(&mut self, table : &mut DynamicPageTable, v_start: VirtualAddr, v_end: VirtualAddr)
    {
        let vpn_start : VirtPageNum = v_start.into();
        let vpn_end : VirtPageNum = v_end.into();
        if vpn_start.0 < vpn_end.0 {
            let index :  usize = (vpn_end.0 - vpn_start.0) >> 21;
            let step : usize = 1 << 9;
            for i in 0..index {
                let s_vpn : VirtPageNum = VirtPageNum::from(vpn_start.0 + i*step);
                let l2_table = table.get_n_level_table_ppn(s_vpn, 2);
                table.clear_l2_entrys(s_vpn, l2_table);
                for j in 0..512 {
                    let cur_vpn : VirtPageNum = VirtPageNum::from(s_vpn.0 + j);
                    self.mem_frames.remove(&cur_vpn);
                }
            }
        } else {
            panic!("unexpected area:0x{:x}--0x{:x}", vpn_start.0, vpn_end.0);
        }
    }

    fn set_frame_mapping(&mut self, valid : bool, table : &mut DynamicPageTable) {
        let addr_start = self.area.vaddr_start.round_down_in_4k();
        let addr_end =  self.area.vaddr_end.round_up_in_4k();
        let addr_start_block = self.area.vaddr_start.round_up_in_2m();
        let addr_end_block = self.area.vaddr_end.round_down_in_2m();
        let pte_flags = RisvPTEFlags::from_bits(self.area.perm.bits).unwrap();
        if addr_start_block.0 < addr_end_block.0 {
            if valid {
                self.set_l3_frames(table, addr_start, addr_start_block, pte_flags);
                //set 2M table:
                self.set_l2_frames(table, addr_start_block, addr_end_block, pte_flags);
                //set lower frames for table:
                self.set_l3_frames(table, addr_end_block, addr_end, pte_flags);
            } else {
                self.free_l3_frames(table, addr_start, addr_start_block);
                self.free_l2_frames(table, addr_start_block, addr_end_block);
                self.free_l3_frames(table, addr_end_block, addr_end);
            }
        } else {
            if valid {
                self.set_l3_frames(table, addr_start, addr_end, pte_flags);
            } else {
                self.free_l3_frames(table, addr_start, addr_end);
            }
        }
    }
    pub fn map_area(&mut self, table : &mut DynamicPageTable) {
        self.set_frame_mapping(true, table);
    }
    pub fn unmap_area(&mut self, table : &mut DynamicPageTable) {
        self.set_frame_mapping(false, table);
    }
    pub fn copy_src_data(&self, app_addr: usize, app_len : usize, v_start: VirtualAddr, v_end: VirtualAddr)
    {
        //first check data size
        if v_end.0 <= v_start.0 {
            panic!("unexpected copy area:0x{:x}--0x{:x}", v_start.0, v_end.0);
        }
        if (v_end.0 - v_start.0) < app_len {
            panic!("unexpected copy size:0x{:x}--0x{:x} vs {}", v_start.0, v_end.0, app_len);
        }
        if (v_start.0 % KERNEL_PAGE_SIZE) != 0 {
            panic!("unexpected start area:0x{:x}", v_start.0);
        }
        //copy data, start addr is always in 4K align
        let start_vpn : VirtPageNum = v_start.into();
        let end_vpn : VirtPageNum = VirtualAddr::from(v_start.0 + app_len).round_up_in_4k().into();
        for index in start_vpn.0..end_vpn.0{
            let src_addr = app_addr + (index-start_vpn.0)*KERNEL_PAGE_SIZE;
            let src = unsafe {core::slice::from_raw_parts(src_addr as *const u8 , KERNEL_PAGE_SIZE)};
            let cur_vpn : VirtPageNum = index.into();
            let frame = self.mem_frames.get(&cur_vpn).unwrap();
            let dst_addr : usize = PhysAddr::from(frame.ppn).into();
            let dst = unsafe {core::slice::from_raw_parts_mut(dst_addr as *mut u8 , KERNEL_PAGE_SIZE)};
            dst.copy_from_slice(src);
        }
    }
}

//user memory maps
pub struct UserMemorySets{
    pub table : DynamicPageTable,
    pub sets : HashMap<String, FrameBasedArea>,
}

impl UserMemorySets{
    pub fn new()->Self {
        Self {
            table : DynamicPageTable::new(),
            sets : HashMap::new(),
        }
    }
    pub fn print_maps(&self) {
        println! ("Root table:0x{:x}", self.table.root_ppn.0);
        for (section, map) in &self.sets {
            println! ("[{}]: 0x{:x}-0x{:x}; perms:{}", section, map.area.vaddr_start.0, map.area.vaddr_end.0, map.area.perm.bits);
        }
    }
    //add dynamic map area for kernel or process
    pub fn add_new_user_map(&mut self, name : String, in_area : MemoryStaticArea) -> bool{
        let mut can_map = true;
        for (section, map) in &self.sets {
            let addr_start = map.area.vaddr_start.0;
            let addr_end = map.area.vaddr_end.0;
            let wanted_start = in_area.vaddr_start.0;
            let wanted_end = in_area.vaddr_end.0;
            if wanted_start >= wanted_end {
                println!("{}: wrong map area:0x{:x}--0x{:x}", name, wanted_start, wanted_end);
                can_map = false;
                break;
            }
            if section.eq(&name) {
                println!("map already exists: name:{}, area:0x{:x}--0x{:x}", name, wanted_start, wanted_end);
                can_map = false;
                break;
            }
            if (wanted_start >= addr_end) || (wanted_end <= addr_start) {
                continue;
            } else {
                can_map = false;
                break;
            }
        }
        if can_map {
            let mut area = FrameBasedArea::new(in_area);
            area.map_area(&mut self.table);
            self.sets.insert(name, area);
        }
        can_map
    }
    pub fn remove_user_map(&mut self, name : &String)-> bool{
        match self.sets.get_mut(name) {
            Some(review) => {
                review.unmap_area(&mut self.table);
                self.sets.remove(name);
                true
            },
            None => {
                println!("{} is not found, just skip.", name);
                false
            }
        }
    }
    pub fn remove_all_map(&mut self) {
        let mut maps : Vec<String> = Vec::with_capacity(self.sets.len());
        for (name, _) in &self.sets {
            maps.push(name.clone());
        }
        for section in maps {
            self.remove_user_map(&section);
        }
    }
    pub fn load_with_elf(&mut self, elf_data: &[u8]) ->usize {
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtualAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtualAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm :MapPermission  = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MemoryStaticArea::new(start_va, end_va, map_perm);
                let name : String = i.to_string();
                self.add_new_user_map(name, map_area);
                let src_begin = ph.offset() as usize + elf_data.as_ptr() as usize;
                let src_len = ph.file_size() as usize;
                let map_name = i.to_string();
                //println!("sections name:{}, area:0x{:x}--0x{:x} dst:0x{:x} len:0x{:x}", map_name, start_va.0, end_va.0, src_begin, src_len);
                let frame_area = self.sets.get(&map_name).unwrap();
                frame_area.copy_src_data(src_begin, src_len, start_va, end_va);
            }
        }
        //add trap text
        self.set_trap_text_page();
        //return user entrypoint
        elf.header.pt2.entry_point() as usize
    }
    pub fn add_user_stack(&mut self, thread_id : usize) {
        let end_va: VirtualAddr = (get_user_stack_top(thread_id) as usize).into();
        let start_va: VirtualAddr = ((end_va.0 - USER_STACK_SIZE) as usize).into();
        let map_perm = MapPermission::U | MapPermission::R | MapPermission::W;
        let map_area = MemoryStaticArea::new(start_va, end_va, map_perm);
        let name : String = String::from("usr_stack");
        self.add_new_user_map(name, map_area);
        //clear area
        let stack : String = String::from("usr_stack");
        self.clear_map_area(&stack, start_va, end_va);
    }
    pub fn add_trap_context(&mut self, thread_id : usize) {
        let start_va: VirtualAddr = get_user_trap_context_start(thread_id).into();
        let end_va: VirtualAddr = (start_va.0 + KERNEL_PAGE_SIZE).into();
        let map_perm = MapPermission::R | MapPermission::W;
        let map_area = MemoryStaticArea::new(start_va, end_va, map_perm);
        let name : String = String::from("trap_context");
        self.add_new_user_map(name, map_area);
        //clear area
        let stack : String = String::from("trap_context");
        self.clear_map_area(&stack, start_va, end_va);
    }
    pub fn clear_map_area(&mut self, name : &String, v_start: VirtualAddr, v_end: VirtualAddr)
    {
        let area = self.sets.get(name).unwrap();
        let start_vpn : VirtPageNum = v_start.round_down_in_4k().into();
        let end_vpn : VirtPageNum =  v_end.round_up_in_4k().into();
        for vpn in start_vpn.0 .. end_vpn.0{
            let temp : VirtPageNum = vpn.into();
            let frame = area.mem_frames.get(&temp).unwrap();
            //phy_addr = kern_addr
            memset_usize(PhysAddr::from(frame.ppn).into(), 0, 512);
        }
    }
    pub fn get_user_start_args_paddr(&self, thread_id : usize) -> usize{
        //only thread 0 has start args
        let start_va: VirtualAddr = (get_user_stack_top(thread_id) - KERNEL_PAGE_SIZE).into();
        let ppn = self.table.do_table_walk(start_va.into());
        PhysAddr::from(ppn.unwrap()).into()
    }
    pub fn get_trap_context_paddr(&self, thread_id : usize)-> usize {
        let start_va: VirtualAddr = get_user_trap_context_start(thread_id).into();
        let ppn = self.table.do_table_walk(start_va.into());
        PhysAddr::from(ppn.unwrap()).into()
    }
    pub fn set_trap_text_page(&mut self)
    {
        extern "C" {
            fn strap();
        }
        let start_va: VirtualAddr = (RISV_TRAP_TEXT_STRAT as usize).into();
        let end_va: VirtualAddr = (USIZE_MAX as usize).into();
        let map_perm = MapPermission::R | MapPermission::X;
        let map_area = MemoryStaticArea::new(start_va, end_va, map_perm);
        let area = FrameBasedArea::new(map_area);
        let ppn : PhysPageNum = PhysAddr::from(strap as usize).into();
        let trap : String = String::from("trap_text");
        self.sets.insert(trap, area);
        self.table.add_an_existed_page(start_va.into(), ppn, RisvPTEFlags::from_bits(map_perm.bits).unwrap());
    }
    pub fn add_framebuffer_addr(&mut self, phy_addrs : usize, fb_len : usize) -> isize{
        let phys_start : PhysAddr = PhysAddr::from(phy_addrs).round_down_in_4k();
        let phys_end : PhysAddr = PhysAddr::from(phy_addrs + fb_len).round_up_in_4k();
        let vaddr_start : VirtualAddr  = VirtualAddr::from(USER_FRAMEBUFFER_MAPPED_ADDR);
        let pages : usize = (phys_end.0 - phys_start.0)/KERNEL_PAGE_SIZE;
        let mut vpn : VirtPageNum =  vaddr_start.into();
        let mut ppn : PhysPageNum = phys_start.into();
        let map_perm = MapPermission::R | MapPermission::W | MapPermission::U;
        for _ in 0..pages {
            self.table.add_an_existed_page(vpn, ppn, RisvPTEFlags::from_bits(map_perm.bits).unwrap());
            vpn.step();
            ppn.step();
        }
        //insert area
        let map_area = MemoryStaticArea::new(vaddr_start, vpn.into(), map_perm);
        let area = FrameBasedArea::new(map_area);
        let trap : String = String::from("framebuffer");
        self.sets.insert(trap, area);
        USER_FRAMEBUFFER_MAPPED_ADDR as isize
    }
    pub fn fork_user_memory(&self, new_set : &mut UserMemorySets) {
        let trap : String = String::from("trap_text");
        for (section, map) in &self.sets {
            if section.eq(&trap) {
                new_set.set_trap_text_page();
                continue;
            }
            let area = map.area;
            let name = section.clone();
            new_set.add_new_user_map(name, area);
            let start_vpn : VirtPageNum = area.vaddr_start.round_down_in_4k().into();
            let end_vpn : VirtPageNum = area.vaddr_end.round_up_in_4k().into();
            let new_map = new_set.sets.get(section).unwrap();
            for page in start_vpn.0..end_vpn.0{
                let vpn : VirtPageNum = page.into();
                let src_frame = map.mem_frames.get(&vpn);
                let dst_frame = new_map.mem_frames.get(&vpn);
                let src_phys : PhysAddr = src_frame.unwrap().ppn.into();
                let dst_phys : PhysAddr = dst_frame.unwrap().ppn.into();
                let src_data = unsafe {core::slice::from_raw_parts(src_phys.0 as *mut usize , 512)};
                let dst_data = unsafe {core::slice::from_raw_parts_mut(dst_phys.0 as *mut usize , 512)};
                dst_data.copy_from_slice(src_data);
            }
        }
    }
}

pub fn get_user_stack_top(thread_id : usize)-> usize {
    RISV_USER_STACK_START + (KERNEL_PAGE_SIZE + USER_STACK_SIZE)*(thread_id+1)
}

pub fn get_user_trap_context_start(thread_id : usize)-> usize {
    RISV_TRAP_CONTEXT_STRAT + KERNEL_PAGE_SIZE*(2*thread_id+1)
}

pub fn translate_user_buffer(buf : usize, len : usize, map : &mut HashMap<usize, PhysBuffer>)
{
    let table = get_current_root_ppn();
    let start_addr : VirtualAddr = buf.into();
    let end_addr : VirtualAddr  = (start_addr.0 + len).into();
    let start_page : VirtualAddr = start_addr.round_down_in_4k();
    let end_page : VirtualAddr = end_addr.round_down_in_4k();
    //default is in one page
    if start_page.0 == end_page.0 {
        let addr : PhysAddr =  do_table_walk_in_4k(table, start_page.into()).unwrap().into();
        let buf_start = start_addr.0 - start_page.0 + addr.0;
        let buf = PhysBuffer::new(buf_start, len);
        map.insert(0, buf);
    } else {
        //check first page
        println!("dozen pages");
        let mut i : usize = 0;
        let addr1 : PhysAddr =  do_table_walk_in_4k(table, start_page.into()).unwrap().into();
        let offset1 : usize = start_addr.0 - start_page.0;
        let buf_start1 = offset1 + addr1.0;
        let buf_0 = PhysBuffer::new(buf_start1, KERNEL_PAGE_SIZE - offset1);
        map.insert(i, buf_0);
        //check middle pages
        let vpn_start : VirtPageNum = start_page.into();
        let vpn_end : VirtPageNum = end_page.into();
        if (vpn_end.0 - vpn_start.0) > 1 {
            for vpn in vpn_start.0 +1 .. vpn_end.0 {
                let addr : PhysAddr =  do_table_walk_in_4k(table, vpn.into()).unwrap().into();
                let buf_i = PhysBuffer::new(addr.0, KERNEL_PAGE_SIZE);
                i += 1;
                map.insert(i, buf_i);
            }
        }
        //check last pages
        let addr2 : PhysAddr =  do_table_walk_in_4k(table, end_page.into()).unwrap().into();
        let offset2 : usize = len + offset1 - (vpn_end.0 - vpn_start.0)*KERNEL_PAGE_SIZE;
        let buf_2 = PhysBuffer::new(addr2.0 , offset2);
        i += 1;
        map.insert(i, buf_2);
    }
}

//a continous buffer with physical address
pub struct PhysBuffer{
    pub start : usize,
    pub len : usize,
}

impl PhysBuffer {
    pub fn new(buf_start : usize, buf_len : usize)->Self {
        Self {
            start : buf_start,
            len : buf_len,
        }
    }
}

// a continous buffer of current user
pub struct UserBuffer{
    pub v_start : usize,
    pub len : usize,
    pub kernel_bufs : HashMap<usize, PhysBuffer>,
}
impl UserBuffer {
    pub fn new(buf_start : usize, buf_len : usize)->Self {
        let mut user_buf = UserBuffer {
            v_start : buf_start,
            len : buf_len,
            kernel_bufs : HashMap::new(),
        };
        user_buf.do_kernel_translate();
        user_buf
    }
    fn do_kernel_translate(&mut self) {
        if self.kernel_bufs.is_empty() {
            translate_user_buffer(self.v_start, self.len, &mut self.kernel_bufs);
        }
    }
    pub fn read_buff_to_kernel_slice(&self, read_buf : usize, read_len : usize) {
        let nums = self.kernel_bufs.len();
        let mut index : usize = 0;
        for i in 0..nums {
            let phys_buf = self.kernel_bufs.get(&i).unwrap();
            let copy_len : usize;
            if read_len < (phys_buf.len + index) {
                copy_len = read_len - index;
            } else {
                copy_len = phys_buf.len;
            }
            let cur_buf = unsafe {core::slice::from_raw_parts(phys_buf.start as *const u8, copy_len)};
            let dst_slice = unsafe {core::slice::from_raw_parts_mut((read_buf + index) as *mut u8, copy_len)};
            dst_slice.copy_from_slice(cur_buf);
            index += phys_buf.len;
            if index >= read_len {
                break;
            }
        }
    }
    pub fn read_buff_to_kernel_string(&self, strs : &mut String) {
        let nums = self.kernel_bufs.len();
        for i in 0..nums {
            let phys_buf = self.kernel_bufs.get(&i).unwrap();
            let cur_buf = unsafe {core::slice::from_raw_parts(phys_buf.start as *const u8, phys_buf.len)};
            let str = core::str::from_utf8(cur_buf).unwrap();
            strs.push_str(str);
        }
    }
    pub fn write_kernel_slice_to_user(&self, write_buf : usize, write_len : usize) {
        let nums = self.kernel_bufs.len();
        let mut index : usize = 0;
        for i in 0..nums {
            let phys_buf = self.kernel_bufs.get(&i).unwrap();
            let copy_len : usize;
            if write_len < (phys_buf.len + index) {
                copy_len = write_len - index;
            } else {
                copy_len = phys_buf.len;
            }
            let cur_buf = unsafe {core::slice::from_raw_parts_mut(phys_buf.start as *mut u8, copy_len)};
            let src_slice = unsafe {core::slice::from_raw_parts((write_buf + index) as *const u8, copy_len)};
            cur_buf.copy_from_slice(src_slice);
            index += phys_buf.len;
            if index >= write_len {
                break;
            }
        }
    }
}
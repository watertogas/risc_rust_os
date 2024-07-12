use hashbrown::HashMap;
use alloc::boxed::Box;
use crate::mm::address::VirtualAddr;
use crate::board::MMIO;
use crate::task::id::IdStore;
use riscv::register::satp;
use core::arch::asm;
use alloc::string::String;
use crate::alloc::string::ToString;
use crate::mm::address::USIZE_MAX;
use crate::config::KERNEL_PAGE_WIDTH_BITS;
use crate::config::AVALIABLE_MEMORY_END;
use crate::config::KERNEL_PAGE_SIZE;
use crate::config::KERNEL_STACK_SIZE;

use crate::mm::memory_set::MEM_IN_1_GB;
pub const RISV_KERNEL_STACK_ARER_START : usize = USIZE_MAX - MEM_IN_1_GB*2 + 1;
pub const RISV_KERNEL_STACK_ARER_END : usize = RISV_KERNEL_STACK_ARER_START + MEM_IN_1_GB;

use crate::mm::memory_set::MemoryStaticArea;
use crate::mm::memory_set::UserMemorySets;
use crate::mm::memory_set::MapPermission;
use crate::mm::page_table::StaticPageTable;

//static memory area for kernel
pub struct KernelMemorySets{
    //physical address of root kernel page table, should be aligned
    pub kern_table : StaticPageTable,
    //static areas for kernel, used for text&rodata&io_mem......
    pub sets : HashMap<String, MemoryStaticArea>,
    //dynamic frame areas for kernel, used for kernel stacks
    pub dyn_memorys : UserMemorySets,
    pub stack_ids : IdStore,
}

impl KernelMemorySets{
    pub fn new(root_pgt: usize) -> Self {
        KernelMemorySets {
            kern_table : StaticPageTable {
                table : root_pgt,
            },
            sets : HashMap::new(),
            dyn_memorys : UserMemorySets::new(),
            stack_ids : IdStore::new(),
        }
    }
    pub fn init_dynamic_mem(&mut self) {
        self.dyn_memorys.table.set_root_ppn(self.kern_table.table);
        self.dyn_memorys.set_trap_text_page();
        self.stack_ids.init();
    }

    pub fn print_maps(&self) {
        println! ("Kernel Root table:0x{:x}", self.kern_table.table);
        for (section, map) in &self.sets {
            println! ("[{}]: 0x{:x}-0x{:x}; perms:{}", section, map.vaddr_start.0, map.vaddr_end.0, map.perm.bits());
        }
        self.dyn_memorys.print_maps();
    }
    //add basic core map area
    pub fn add_core_map(&mut self, name : String, area : MemoryStaticArea)
    {
        println!("add_core_map: {}", name);
        self.sets.insert(name, area);
    }
    #[allow(unused)]
    //add dynamic map area for kernel
    pub fn add_new_map(&mut self, name : String, area : MemoryStaticArea) -> bool{
        let mut can_map = true;
        for (section, map) in &self.sets {
            let addr_start = map.vaddr_start.0;
            let addr_end = map.vaddr_end.0;
            let wanted_start = area.vaddr_start.0;
            let wanted_end = area.vaddr_end.0;
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
            self.sets.insert(name, area);
            area.map_area_normal(&self.kern_table);
        }
        can_map
    }
    #[allow(unused)]
    pub fn remove_map(&mut self, name : String)-> bool{
        match self.sets.get(&name) {
            Some(review) => {
                review.unmap_area_normal(&self.kern_table);
                self.sets.remove(&name);
                true
            },
            None => {
                println!("{} is not found, just skip.", name);
                false
            }
         }
    }
    pub fn add_kernel_stack(&mut self, stack_id : usize) {
        let kern_stack_start : usize = RISV_KERNEL_STACK_ARER_START + (KERNEL_STACK_SIZE + KERNEL_PAGE_SIZE)*stack_id;
        let kern_stack_end : usize = kern_stack_start + KERNEL_STACK_SIZE + KERNEL_PAGE_SIZE;
        if kern_stack_end > RISV_KERNEL_STACK_ARER_END {
            panic!("unexpected pid with kernel stack:0x{:x}", stack_id);
        }
        let start_va: VirtualAddr = (kern_stack_start + KERNEL_PAGE_SIZE).into();
        let end_va: VirtualAddr = kern_stack_end.into();
        let map_perm = MapPermission::R | MapPermission::W;
        let map_area = MemoryStaticArea::new(start_va, end_va, map_perm);
        let name : String = stack_id.to_string();
        self.dyn_memorys.add_new_user_map(name, map_area);
        //clear kernel stack
        let stack : String = stack_id.to_string();
        self.dyn_memorys.clear_map_area(&stack, start_va, end_va);
    }
    pub fn remove_kernel_stack(&mut self, stack_id : usize) {
        let name : String = stack_id.to_string();
        self.dyn_memorys.remove_user_map(&name);
    }
}

fn open_mmu(token :  usize) {
    unsafe {
        satp::write(token);
        asm!("sfence.vma");
        println!("mmu is opened");
    }
}

static mut KERNEL_MEMSETS: Option<&mut KernelMemorySets> = None;

fn set_memory_sets(boot_level : usize)
{
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn ebss();
        fn ekernel();
        fn kernel_pte_bottom();
    }
    let area1 = MemoryStaticArea::new((stext as usize).into(), (etext as usize).into(), MapPermission::R | MapPermission::X);
    let area2 = MemoryStaticArea::new((srodata as usize).into(), (erodata as usize).into(), MapPermission::R);
    let area3 = MemoryStaticArea::new((sdata as usize).into(), (ebss as usize).into(), MapPermission::R | MapPermission::W);
    let area4 = MemoryStaticArea::new((ekernel as usize).into(), (AVALIABLE_MEMORY_END as usize).into(), MapPermission::R | MapPermission::W);
    
    match boot_level {
        1 => {
            let kern_table = StaticPageTable {
                table : kernel_pte_bottom as usize
            };
            kern_table.check_block_table_aligned();
            area1.map_area_normal(&kern_table);
            area2.map_area_normal(&kern_table);
            area3.map_area_normal(&kern_table);
            area4.map_area_normal(&kern_table);
            for (start , size) in MMIO {
                let cur_area = MemoryStaticArea::new((*start).into(), (*start + *size).into(), MapPermission::R | MapPermission::W);
                cur_area.map_area_normal(&kern_table);
            }
            let base_pgt = kern_table.table;
            let token = (8usize << 60) | (base_pgt >> KERNEL_PAGE_WIDTH_BITS);
            println!("token is 0x{:x}", token);
            open_mmu(token);
        },
        2 => {
            unsafe {
                //kernel heap is avaliable now, just use it 
                let sets = Box::new(KernelMemorySets::new(kernel_pte_bottom as usize));
                KERNEL_MEMSETS = Some(Box::leak(sets));
                KERNEL_MEMSETS.as_mut().unwrap().add_core_map(String::from("text"), area1);
                KERNEL_MEMSETS.as_mut().unwrap().add_core_map(String::from("rodata"), area2);
                KERNEL_MEMSETS.as_mut().unwrap().add_core_map(String::from("data_bss"), area3);
                KERNEL_MEMSETS.as_mut().unwrap().add_core_map(String::from("frames"), area4);
                for i in 0..MMIO.len() {
                    let cur_area = MemoryStaticArea::new(MMIO[i].0.into(), (MMIO[i].0 + MMIO[i].1).into(), MapPermission::R | MapPermission::W);
                    let io_name = String::from("MMIO_") + &(i.to_string());
                    KERNEL_MEMSETS.as_mut().unwrap().add_core_map(io_name, cur_area);
                }
                KERNEL_MEMSETS.as_mut().unwrap().init_dynamic_mem();
                KERNEL_MEMSETS.as_mut().unwrap().print_maps();
            }
        },
        _ => {
            panic!("Unsupported fd in level: {}", boot_level);
        }
    }
}

//This should be used in first boot when mmu is off
pub fn init_first_kernel_mapping()
{
    set_memory_sets(1);
}


//This should be used in second boot when mmu is on
pub fn init_second_kernel_mapping()
{
    set_memory_sets(2);
}

//
pub fn get_kernel_stack_top(stack_id : usize)-> usize {
    RISV_KERNEL_STACK_ARER_START + (KERNEL_STACK_SIZE + KERNEL_PAGE_SIZE)*(stack_id+1)
}


pub fn alloc_kernel_stack()->KernelStack
{
    unsafe {
        let stack = KERNEL_MEMSETS.as_mut().unwrap().stack_ids.alloc_avaliabe_id();
        KERNEL_MEMSETS.as_mut().unwrap().add_kernel_stack(stack);
        KernelStack{
            stack_id : stack,
        }
    }
}

pub fn remove_stack(stack_id : usize)
{
    unsafe {
        KERNEL_MEMSETS.as_mut().unwrap().stack_ids.recycle(stack_id);
        KERNEL_MEMSETS.as_mut().unwrap().remove_kernel_stack(stack_id)
    }
}

pub fn get_kernel_stap()->usize
{
    unsafe {
        KERNEL_MEMSETS.as_mut().unwrap().dyn_memorys.table.get_root_stap()
    }
}

//A RAII kernel stack for user
pub struct KernelStack{
    pub stack_id : usize,
}
impl Drop for KernelStack {
    fn drop(&mut self) {
        remove_stack(self.stack_id);
    }
}
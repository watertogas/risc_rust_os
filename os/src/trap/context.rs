use riscv::register::sstatus::Sstatus;
#[repr(C)]
pub struct TrapContext{
    //common registers
    pub cr : [usize ; 32],
    //regester:sepc:32
    pub sepc: usize,
    //regester:Sstatus:33
    pub sstatus: Sstatus,
    //kernel_stack: index:34
    pub kernel_stack: usize,
    //kernel_stap: index:35
    pub kernel_stap : usize,
    //user_stap: index:36
    pub user_stap : usize,
    ///user_trap_hander absolutr addr, this is mandatory,beacuse we
    /// can not use relative jump when switching ttbr between kernel
    /// and user; index:37
    pub trap_handler: usize,
    ///context physical addr in Kernel or userspace,index:38
    pub context_addr : usize,
}

impl TrapContext {
    pub fn print_trap_context(&self) {
        for i in 0..32 {
            println!("registers[{}]=0x{:x}.", i, self.cr[i]);
        }
        println!("sepc=0x{:x}.", self.sepc);
        println!("sstatus=0x{:x}.", self.sstatus.bits());
        println!("kernel_stack=0x{:x}.", self.kernel_stack);
        println!("kernel_stap=0x{:x}.", self.kernel_stap);
        println!("user_stap=0x{:x}.", self.user_stap);
        println!("trap_handler=0x{:x}.", self.trap_handler);
        println!("context_addr=0x{:x}.", self.context_addr);
    }
}
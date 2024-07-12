use alloc::boxed::Box;
use alloc::vec::Vec;
pub struct IdStore {
    pub max_id : usize,
    pub avalible : Vec<usize>,
}

//implement a auto recycled id vector for Kernel
impl IdStore {
    pub fn new() ->Self {
        Self {
            max_id : 0,
            avalible : Vec::new(),
        }
    }
    //push 4 values in empty store
    pub fn init(&mut self) {
        for i in (0..4).rev() {
            self.avalible.push(i);
        }
        self.max_id = 4;
    }
    fn enlarge(&mut self) {
        let cur = self.max_id;
        self.max_id = cur *2;
        for i in cur..self.max_id{
            self.avalible.push(i);
        }
    }
    pub fn recycle(&mut self, id : usize) {
        self.avalible.push(id)
    }
    pub fn alloc_avaliabe_id(&mut self)->usize{
        if self.avalible.is_empty() {
            self.enlarge();
        }
        self.avalible.pop().unwrap()
    }
    #[allow(unused)]
    pub fn print_unused(&self) {
        println!("max_id: {}", self.max_id);
        for x in &self.avalible {
            println!("avalible: {}", x);
        }
    }
}

pub struct IdWrapper {
    pub id : usize,
}
impl IdWrapper {
    pub fn new(value : usize) ->Self {
        Self {
            id : value,
        }
    }
}

impl Drop for IdWrapper {
    fn drop(&mut self) {
        remove_pid(self.id);
    }
}

static mut PIDSETS: Option<&mut IdStore> = None;

pub fn init_id_sets()
{
    let pid_sets = Box::new(IdStore::new());
    unsafe {
        PIDSETS = Some(Box::leak(pid_sets));
        PIDSETS.as_mut().unwrap().init();
    }
}

pub fn alloc_pid()->IdWrapper{
    unsafe {
        let pid = PIDSETS.as_mut().unwrap().alloc_avaliabe_id();
        IdWrapper::new(pid)
    }
}

fn remove_pid(pid : usize) {
    unsafe {
        PIDSETS.as_mut().unwrap().recycle(pid)
    }
}
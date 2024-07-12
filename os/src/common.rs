
///some copy functions with C style
///Warning: it is unsafe!!!
pub fn memset_u8(start : usize, value : u8, len : usize)
{
    let end = start + len;
    (start..end).for_each(|a|{
        unsafe{(a as *mut u8).write_volatile(value)}
    })
}
pub fn memset_usize(start : usize, value : usize, len : usize)
{
    let area = unsafe {core::slice::from_raw_parts_mut(start as *mut usize , len)};
    for i in 0..len{
        area[i] = value;
    }
}
//! Process management syscalls
use core::mem::size_of;
use crate::{config::MAX_SYSCALL_NUM, mm::{translated_byte_buffer, PTEFlags, PageTable, VirtAddr}, task::{change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_count, mmap, munmap, suspend_current_and_run_next}, timer::get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let tus = get_time_us();
    let time_val = TimeVal {
        sec: tus / 1_000_000,
        usec: tus % 1_000_000,
    };
    // ts所占的所有页上的内存
    let buffers = translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    let pt = PageTable::from_token(token);
    let vpn = VirtAddr::from(id).floor();
    let pte = pt.translate(vpn);
    match trace_request {
        0 => {
            if pte.is_none() || !pte.unwrap().readable() || !pte.unwrap().flags().contains(PTEFlags::U) {
                return -1;
            }
            let buffers = translated_byte_buffer(token, id as *const u8, 1);
            if buffers.is_empty() { -1 } else { buffers[0][0] as isize }
        }
        1 => {
            if pte.is_none() || !pte.unwrap().writable() || !pte.unwrap().flags().contains(PTEFlags::U) {
                return -1;
            }
            let mut buffers = translated_byte_buffer(token, id as *mut u8, 1);
            if buffers.is_empty() {
                return -1;
            }
            buffers[0][0] = (data & 0xFF) as u8;
            0
        }
        2 => {
            if id > MAX_SYSCALL_NUM {
                return -1;
            }
            get_syscall_count(id) as isize
        }
        _ => -1,
    }
}

///
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let len = (len + 0xfff) & !0xfff;
    if port & 0x7 == 0 {
        return -1;
    }
    if start & 0xfff != 0 || port & !(0b111 as usize) != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if mmap(start, len, port) {0} else {-1}
}

///
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if len == 0 {
        return 0;
    }
    let len = (len + 0xfff) & !0xfff;
    if munmap(start, len) {0} else {-1}
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

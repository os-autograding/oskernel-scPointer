//! 专门处理 linux 特色的 futex 锁。
//!
//! 在 rCore-Tutorial 中，把锁、信号量、条件变量机制分别用很多不同的 syscall 来实现，
//! 而在 linux 中都浓缩在了以 futex 为核心的几个 syscall 上，并统一了语义，
//! 具体的机制区别由用户态的库完成，只有当发送冲突时才进入内核。
//!
//! 由于 futex 参数复杂，所以特别开了一个子模块来放和它相关的实现与 flag

mod flags;
mod waiter;
pub use waiter::{Waiter, FutexWaiter};
mod waiting_board;
pub use waiting_board::{check_thread_blocked, wake_thread, set_waiter_for_thread};

use flags::{Flags, FutexFlag};
use lock::Mutex;
use alloc::boxed::Box;
use super::{sys_gettid, ErrorNo, SysResult};
use crate::task::{get_current_task, suspend_current_task};
use crate::timer::{TimeSpec, TimeVal};


static FCOUNT: Mutex<usize> = Mutex::new(0);

pub fn sys_futex(
    uaddr: usize,
    futex_op: i32,
    val: u32,
    val2: usize,
    uaddr2: usize,
    val3: u32,
) -> SysResult {
    let flag = FutexFlag::new(futex_op);
    let tid = sys_gettid().unwrap();
    info!("now tid {}", tid);
    info!(
        "futex: uaddr {:x}, op {} val {} val2 {:x} uaddr2 {:x} val3 {}",
        uaddr, futex_op, val, val2, uaddr2, val3
    );
    if !flag.is_private() { // 不支持跨地址空间
         //panic!("futex not private");
    }

    *FCOUNT.lock() += 1;
    //if uaddr == 0x85f60 && tid == 3 && *FCOUNT.lock() > 300 {
    //    panic!("futex limit");
    //}
    match flag.operation() {
        Flags::WAIT => {
            //info!("futex wait, suspend---");
            // 检查 uaddr 处的地址
            let task = get_current_task().unwrap();
            let mut task_vm = task.vm.lock();
            if task_vm.manually_alloc_page(uaddr).is_ok() {
                let real_val = unsafe { (uaddr as *const u32).read_volatile() };
                if real_val != val {
                    Err(ErrorNo::EAGAIN)
                } else {
                    // 如果是个表示 timeout 的地址
                    let timed_out = if val2 != 0 && task_vm.manually_alloc_page(val2 as usize).is_ok() {
                        let time_spec: TimeSpec = unsafe { *(val2 as *const TimeSpec) };
                        let time_us: usize = TimeVal::from(time_spec).into();
                        info!("futex timed out {time_us} us");
                        Some(time_us)
                        
                        //panic!("");
                    } else {
                        // None，永不通过超时唤醒
                        Some(0)
                    };
                    set_waiter_for_thread(tid, Box::new(FutexWaiter::new(timed_out)));
                    drop(task_vm); // 切换任务前取消对锁的占用
                    drop(task);
                    suspend_current_task();
                    Ok(0)
                }
            } else {
                // 若地址无效
                Err(ErrorNo::EFAULT)
            }
        }
        Flags::WAKE => {
            //info!("futex wake");
            suspend_current_task();
            Ok(val as usize)
        }
        _ => Err(ErrorNo::EINVAL),
    }
}

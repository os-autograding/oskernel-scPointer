//! 系统调用实现
//!
//! 目前的系统调用规范参照比赛所提供的类似 Linux 系统调用实现。
//!
//! 有一些注释的系统调用名，那些是发现应用需要，但尚未实现暂时跳过的系统调用
//!
//! 这两种调用间比较容易混淆的区别是，比赛测例是用 C 写的，大部分数组都是 4 Byte，
//! 而 rCore 使用 rust，usize/isize 一般是 8 Byte。
//! 这导致一些传入地址(非字符串,字符串大家都是统一的 1Byte 类型)的大小有问题，
//! 如 sys_pipe() 在测例环境下需要将输入作为 *mut u32 而不是 *mut usize

//#![deny(missing_docs)]

mod flags;
mod fs;
mod futex;
mod loops;
mod process;
mod select;
mod socket;
mod syscall_no;
mod times;

pub use flags::{PollFd, ErrorNo};
use flags::*;
use fs::*;
use futex::*;
pub use futex::{check_thread_blocked, wake_thread, set_waiter_for_thread};
use loops::*;
pub use loops::clear_loop_checker;
use process::*;
use select::*;
use socket::*;
use syscall_no::SyscallNo;
use times::*;

use crate::file::{FsStat, Kstat, EpollEvent};
use crate::signal::SigAction;
use crate::task::ITimerVal;
use crate::timer::{TimeSpec, TimeVal};

type SysResult = Result<usize, ErrorNo>;

/// 处理系统调用
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    check_dead_loop(syscall_id);
    let syscall_id = if let Ok(id) = SyscallNo::try_from(syscall_id) {
        id
    } else {
        error!(
            "Unsupported syscall id = {:#?}({})",
            syscall_id, syscall_id as usize
        );
        return 0;
    };
    debug!("Syscall {:?}, {:x?}", syscall_id, args);

    let result = match syscall_id {
        SyscallNo::GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SyscallNo::EPOLL_CREATE => sys_epoll_create(args[0]),
        SyscallNo::EPOLL_CTL => sys_epoll_ctl(args[0] as i32, args[1] as i32, args[2] as i32, args[3] as *const EpollEvent),
        SyscallNo::EPOLL_WAIT => sys_epoll_wait(args[0] as i32, args[1] as *mut EpollEvent, args[2] as i32, args[3] as i32),
        SyscallNo::DUP => sys_dup(args[0]),
        SyscallNo::DUP3 => sys_dup3(args[0], args[1]),
        SyscallNo::FCNTL64 => sys_fcntl64(args[0], args[1], args[2]),
        SyscallNo::UNLINKAT => sys_unlinkat(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SyscallNo::LINKAT => sys_linkat(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as i32,
            args[3] as *const u8,
            args[4] as u32,
        ),
        SyscallNo::UMOUNT => sys_umount(args[0] as *const u8, args[1] as u32),
        SyscallNo::MOUNT => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3] as u32,
            args[4] as *const u8,
        ),
        SyscallNo::STATFS => sys_statfs(args[0] as *const u8, args[1] as *mut FsStat),
        SyscallNo::ACCESS => sys_access(args[0] as i32, args[1] as *const u8, args[2]),
        SyscallNo::MKDIR => sys_mkdir(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SyscallNo::CHDIR => sys_chdir(args[0] as *const u8),
        SyscallNo::OPEN => sys_open(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as u32,
            args[3] as i32,
        ),
        SyscallNo::CLOSE => sys_close(args[0]),
        SyscallNo::PIPE => sys_pipe(args[0] as *mut u32),
        SyscallNo::GETDENTS64 => sys_getdents64(args[0], args[1] as *mut u8, args[2]),
        SyscallNo::LSEEK => sys_lseek(args[0], args[1] as isize, args[2] as isize),
        SyscallNo::READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        SyscallNo::WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SyscallNo::READV => sys_readv(args[0], args[1] as *mut IoVec, args[2]),
        SyscallNo::WRITEV => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        SyscallNo::PREAD => sys_pread(args[0], args[1] as *mut u8, args[2], args[3]),
        SyscallNo::SENDFILE64 => sys_sendfile64(args[0], args[1], args[2] as *mut usize, args[3]),
        SyscallNo::PSELECT6 => sys_pselect6(
            args[0],
            args[1] as *mut usize,
            args[2] as *mut usize,
            args[3] as *mut usize,
            args[4] as *const TimeSpec,
            args[5] as *const usize,
        ),
        SyscallNo::PPOLL => sys_ppoll(
            args[0] as *mut PollFd,
            args[1],
            args[2] as *const TimeSpec,
            args[3] as *const usize,
        ),
        SyscallNo::READLINKAT => sys_readlinkat(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as *mut u8,
            args[3],
        ),
        SyscallNo::FSTATAT => sys_fstatat(args[0] as i32, args[1] as *const u8, args[2] as *mut Kstat),
        SyscallNo::FSTAT => sys_fstat(args[0], args[1] as *mut Kstat),
        SyscallNo::UTIMENSAT => sys_utimensat(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as *const TimeSpec,
            UtimensatFlags::from_bits(args[3] as u32).unwrap(),
        ),
        SyscallNo::EXIT => sys_exit(args[0] as i32),
        SyscallNo::EXIT_GROUP => sys_exit(args[0] as i32),
        SyscallNo::SET_TID_ADDRESS => sys_set_tid_address(args[0]),
        SyscallNo::FUTEX => sys_futex(
            args[0],
            args[1] as i32,
            args[2] as u32,
            args[3],
            args[4],
            args[5] as u32,
        ),
        SyscallNo::NANOSLEEP => sys_nanosleep(args[0] as *const TimeSpec, args[1] as *mut TimeSpec),
        SyscallNo::GETITIMER => sys_gettimer(args[0], args[1] as *mut ITimerVal),
        SyscallNo::SETITIMER => sys_settimer(
            args[0],
            args[1] as *const ITimerVal,
            args[2] as *mut ITimerVal,
        ),
        SyscallNo::CLOCK_GET_TIME => sys_clock_gettime(args[0], args[1] as *mut TimeSpec),
        SyscallNo::YIELD => sys_yield(),
        SyscallNo::KILL => sys_kill(args[0] as isize, args[1] as isize),
        SyscallNo::TKILL => sys_tkill(args[0] as isize, args[1] as isize),
        SyscallNo::SIGACTION => sys_sigaction(
            args[0],
            args[1] as *const SigAction,
            args[2] as *mut SigAction,
        ),
        SyscallNo::SIGPROCMASK => sys_sigprocmask(
            args[0] as i32,
            args[1] as *const usize,
            args[2] as *mut usize,
            args[3],
        ),
        SyscallNo::SIGRETURN => sys_sigreturn(),
        SyscallNo::TIMES => sys_times(args[0] as *mut TMS),
        SyscallNo::UNAME => sys_uname(args[0] as *mut UtsName),
        SyscallNo::GETRUSAGE => sys_getrusage(args[0] as i32, args[1] as *mut TimeVal),
        SyscallNo::UMASK => sys_umask(args[0] as i32),
        SyscallNo::GET_TIME_OF_DAY => sys_get_time_of_day(args[0] as *mut TimeVal),
        SyscallNo::GETPID => sys_getpid(),
        SyscallNo::GETPPID => sys_getppid(),
        SyscallNo::GETUID => sys_getuid(),
        SyscallNo::GETEUID => sys_geteuid(),
        SyscallNo::GETGID => sys_getgid(),
        SyscallNo::GETEGID => sys_getegid(),
        SyscallNo::GETTID => sys_gettid(),
        SyscallNo::SYSINFO => sys_sysinfo(args[0] as *mut SysInfo),
        SyscallNo::SOCKET => sys_socket(args[0], args[1], args[2]),
        SyscallNo::SENDTO => sys_sendto(
            args[0],
            args[1] as *const u8,
            args[2],
            args[3] as i32,
            args[4] as *const u8,
            args[5],
        ),
        SyscallNo::RECVFROM => sys_recvfrom(
            args[0],
            args[1] as *mut u8,
            args[2],
            args[3] as i32,
            args[4] as *mut u8,
            args[5] as *mut u32,
        ),
        SyscallNo::BIND => sys_bind(args[0], args[1] as *const u8, args[2]),
        SyscallNo::LISTEN => sys_listen(args[0], args[1]),
        SyscallNo::CONNECT => sys_connect(args[0], args[1] as *const u8, args[2]),
        SyscallNo::ACCEPT => sys_accept4(args[0], args[1] as *mut u8, args[2] as *mut u32, args[3] as i32),
        SyscallNo::ACCEPT4 => sys_accept4(args[0], args[1] as *mut u8, args[2] as *mut u32, args[3] as i32),
        SyscallNo::BRK => sys_brk(args[0]),
        SyscallNo::MUNMAP => sys_munmap(args[0], args[1]),
        SyscallNo::CLONE => sys_clone(args[0], args[1], args[2], args[3], args[4]),
        SyscallNo::MMAP => sys_mmap(
            args[0],
            args[1],
            MMAPPROT::from_bits(args[2] as u32).unwrap(),
            MMAPFlags::from_bits(args[3] as u32).unwrap(),
            args[4] as i32,
            args[5],
        ),
        SyscallNo::MPROTECT => sys_mprotect(
            args[0],
            args[1],
            MMAPPROT::from_bits(args[2] as u32).unwrap(),
        ),
        SyscallNo::MSYNC => sys_msync(args[0], args[1], MSyncFlags::from_bits(args[2] as u32).unwrap()),
        SyscallNo::EXECVE => sys_execve(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),
        SyscallNo::WAIT4 => sys_wait4(
            args[0] as isize,
            args[1] as *mut i32,
            WaitFlags::from_bits(args[2] as u32).unwrap(),
        ),
        SyscallNo::PRLIMIT64 => sys_prlimt64(
            args[0],
            args[1] as i32,
            args[2] as *const RLimit,
            args[3] as *mut RLimit,
        ),
        SyscallNo::RENAMEAT2 => sys_renameat2(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as i32,
            args[3] as *const u8,
            RenameFlags::from_bits(args[4] as u32).unwrap()
        ),
        SyscallNo::IOCTL => sys_ioctl(args[0], args[1], args[2] as *mut usize),
        //SyscallNo::MPROTECT => 0,
        SyscallNo::SIGTIMEDWAIT => Ok(0),
        SyscallNo::MEMBARRIER => Ok(0),
        SyscallNo::FSYNC => Ok(0),
        _ => {
            //_ => panic!("Unsupported syscall id = {:#?}()", syscall_id, syscall_id as usize);
            warn!(
                "Unsupported syscall id = {:#?}({})",
                syscall_id, syscall_id as usize
            );
            Ok(0)
        }
    };
    match result {
        Ok(a0) => {
            if syscall_id != SyscallNo::GETRUSAGE
                && syscall_id != SyscallNo::CLOCK_GET_TIME
            {
                debug!("{:?} ret -> {} = {:#x}", syscall_id, a0, a0);
            }
            a0 as isize
        }
        Err(num) => {
            warn!("{:?} ret -> {:?}", syscall_id, num);
            num as isize
        }
    }
}

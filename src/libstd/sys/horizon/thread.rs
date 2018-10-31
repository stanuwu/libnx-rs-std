// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(not(target_arch = "aarch64"))]
mod nds {
    use alloc_crate::boxed::FnBox;
    use cmp;
    use ffi::CStr;
    use io;
    use libc;
    use libctru::Thread as ThreadHandle;
    use mem;
    use ptr;
    use sys_common::thread::start_thread;
    use time::Duration;

    pub struct Thread {
        handle: ThreadHandle,
    }

    unsafe impl Send for Thread {}
    unsafe impl Sync for Thread {}

    pub const DEFAULT_MIN_STACK_SIZE: usize = 4096;

    impl Thread {
        pub unsafe fn new<'a>(stack: usize, p: Box<FnBox() + 'a>) -> io::Result<Thread> {
            let p = box p;
            let stack_size = cmp::max(stack, DEFAULT_MIN_STACK_SIZE);

            let mut priority = 0;
            ::libctru::svcGetThreadPriority(&mut priority, 0xFFFF8000);

            let handle = ::libctru::threadCreate(
                Some(thread_func),
                &*p as *const _ as *mut _,
                stack_size,
                priority,
                -2,
                false,
            );

            return if handle == ptr::null_mut() {
                Err(io::Error::from_raw_os_error(libc::EAGAIN))
            } else {
                mem::forget(p); // ownership passed to the new thread
                Ok(Thread { handle: handle })
            };

            extern "C" fn thread_func(start: *mut libc::c_void) {
                unsafe { start_thread(start as *mut u8) }
            }
        }

        pub fn yield_now() {
            unsafe { ::libctru::svcSleepThread(0) }
        }

        pub fn set_name(_name: &CStr) {
            // threads aren't named in libctru
        }

        pub fn sleep(dur: Duration) {
            unsafe {
                let nanos = dur
                    .as_secs()
                    .saturating_mul(1_000_000_000)
                    .saturating_add(dur.subsec_nanos() as u64);
                ::libctru::svcSleepThread(nanos as i64)
            }
        }

        pub fn join(self) {
            unsafe {
                let ret = ::libctru::threadJoin(self.handle, u64::max_value());
                ::libctru::threadFree(self.handle);
                mem::forget(self);
                debug_assert_eq!(ret, 0);
            }
        }

        #[allow(dead_code)]
        pub fn id(&self) -> ThreadHandle {
            self.handle
        }

        #[allow(dead_code)]
        pub fn into_id(self) -> ThreadHandle {
            let handle = self.handle;
            mem::forget(self);
            handle
        }
    }

    impl Drop for Thread {
        fn drop(&mut self) {
            unsafe { ::libctru::threadDetach(self.handle) }
        }
    }

    pub mod guard {
        pub unsafe fn current() -> Option<usize> {
            None
        }
        pub unsafe fn init() -> Option<usize> {
            None
        }
    }
}
#[cfg(not(target_arch = "aarch64"))]
pub use self::nds::*;

#[cfg(target_arch = "aarch64")]
mod switch {
    use alloc_crate::boxed::FnBox;
    use cmp;
    use ffi::CStr;
    use io;
    use mem;
    use ptr;
    use sys_common::thread::start_thread;
    use time::Duration;
    use cell::UnsafeCell;
    use libnx_rs::libnx::Thread as SThread;
    use libc;

    #[repr(C)]
    pub struct ThreadHandle {
        handle: SThread,
        rc : i32
    }

    pub struct Thread {
        handle : UnsafeCell<ThreadHandle>
    }

    unsafe impl Send for Thread {}
    unsafe impl Sync for Thread {}

    pub const DEFAULT_MIN_STACK_SIZE: usize = 4096;

    #[repr(C)]
    struct C_TimeSpec {
        tv_sec : u64, 
        tv_nsec : u64
    }
    extern {
        fn thrd_create(thr : *mut *mut ThreadHandle, func : extern fn(*mut libc::c_void) -> i32, arg : *mut libc::c_void) -> i32;
        fn thrd_yield();
        fn thrd_sleep(dur : *const C_TimeSpec, rem : *mut C_TimeSpec) -> i32;
        fn thrd_exit(res : i32);
        fn thrd_join(thr : *mut ThreadHandle, res : *mut i32) -> i32; 
    }

    impl Thread {
        pub unsafe fn new<'a>(stack: usize, p: Box<FnBox() + 'a>) -> io::Result<Thread> {

            let handle_mem : UnsafeCell<ThreadHandle> = UnsafeCell::new(mem::zeroed());
            let mut handle_ptr = handle_mem.get();
            let rs = thrd_create(&mut handle_ptr as *mut *mut ThreadHandle, thread_func, &p as *const Box<FnBox() + 'a> as *mut Box<FnBox() + 'a> as *mut libc::c_void);

            return match rs {
                1 => {
                    Err(io::Error::new(io::ErrorKind::Other, "Thread busy!"))
                },
                2 => {
                    Err(io::Error::new(io::ErrorKind::Other, "Thread error!"))
                },
                3 => {
                    Err(io::Error::new(io::ErrorKind::Other, "Thread nomem!"))
                },
                4 => {
                    mem::forget(p); // ownership passed to the new thread
                    Ok(Thread { handle: handle_mem })
                },
                5 => {
                    Err(io::Error::new(io::ErrorKind::Other, "Thread timeout!"))
                },
                e => {
                    Err(io::Error::new(io::ErrorKind::Other, format!("Thread create retval: {}!", e)))
                }
            };

            extern "C" fn thread_func(start: *mut libc::c_void) -> i32 {
                unsafe { start_thread(start as *mut u8) };
                0
            }
        }

        pub fn yield_now() {
            unsafe {thrd_yield()};
        }

        pub fn set_name(_name: &CStr) {
            // threads aren't named in libctru
        }

        pub fn sleep(dur: Duration) {
            unsafe {
                let dur = C_TimeSpec {
                    tv_sec : dur.as_secs(), 
                    tv_nsec : dur.subsec_nanos() as u64
                };
                thrd_sleep(&dur as *const C_TimeSpec, ptr::null_mut());
            }
        }

        pub fn join(mut self) {
            unsafe {
                let mut res = 0;
                thrd_join(self.handle.get(), &mut res as *mut i32);
            }
        }

        #[allow(dead_code)]
        pub fn id(&self) -> ThreadHandle {
            unsafe { mem::transmute_copy(&self.handle) }
        }

        #[allow(dead_code)]
        pub fn into_id(self) -> ThreadHandle {
            let handle = unsafe { mem::transmute_copy(&self.handle) };
            mem::forget(self);
            handle
        }
    }

    impl Drop for Thread {
        fn drop(&mut self) {
            //TODO: kill the thread
        }
    }

    pub mod guard {
        pub unsafe fn current() -> Option<usize> {
            None
        }
        pub unsafe fn init() -> Option<usize> {
            None
        }
    }

}

#[cfg(target_arch = "aarch64")]
pub use self::switch::*;

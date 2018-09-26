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
    use cell::UnsafeCell;
    use mem;
    pub struct Mutex {
        inner: UnsafeCell<::libctru::LightLock>,
    }

    #[inline]
    pub unsafe fn raw(m: &Mutex) -> *mut ::libctru::LightLock {
        m.inner.get()
    }

    unsafe impl Send for Mutex {}
    unsafe impl Sync for Mutex {}

    impl Mutex {
        pub const fn new() -> Mutex {
            Mutex { inner: UnsafeCell::new(0) }
        }

        #[inline]
        pub unsafe fn init(&mut self) {
            ::libctru::LightLock_Init(self.inner.get());
        }

        #[inline]
        pub unsafe fn lock(&self) {
            ::libctru::LightLock_Lock(self.inner.get());
        }

        #[inline]
        pub unsafe fn unlock(&self) {
            ::libctru::LightLock_Unlock(self.inner.get());
        }

        #[inline]
        pub unsafe fn try_lock(&self) -> bool {
            match ::libctru::LightLock_TryLock(self.inner.get()) {
                0 => false,
                _ => true,
            }
        }

        #[inline]
        pub unsafe fn destroy(&self) {
        }
    }

    pub struct ReentrantMutex { inner: UnsafeCell<::libctru::RecursiveLock> }

    unsafe impl Send for ReentrantMutex {}
    unsafe impl Sync for ReentrantMutex {}

    impl ReentrantMutex {
        pub unsafe fn uninitialized() -> ReentrantMutex {
            ReentrantMutex { inner: mem::uninitialized() }
        }

        pub unsafe fn init(&mut self) {
            ::libctru::RecursiveLock_Init(self.inner.get());
        }

        pub unsafe fn lock(&self) {
            ::libctru::RecursiveLock_Lock(self.inner.get());
        }

        #[inline]
        pub unsafe fn try_lock(&self) -> bool {
            match ::libctru::RecursiveLock_TryLock(self.inner.get()) {
                0 => false,
                _ => true,
            }
        }

        pub unsafe fn unlock(&self) {
            ::libctru::RecursiveLock_Unlock(self.inner.get());
        }

        pub unsafe fn destroy(&self) {}
    }
}
#[cfg(not(target_arch = "aarch64"))]
pub use self::nds::*;

#[cfg(target_arch = "aarch64")]
mod switch {
    use cell::UnsafeCell;
    use mem;

    use libnx_rs::libnx;

    pub struct Mutex {
        inner: UnsafeCell<libnx::Mutex>,
    }

    #[inline]
    pub unsafe fn raw(m: &Mutex) -> *mut libnx::Mutex {
        m.inner.get()
    }

    unsafe impl Send for Mutex {}
    unsafe impl Sync for Mutex {}

    #[cfg(target_arch = "aarch64")]
    impl Mutex {
        pub const fn new() -> Mutex {
            Mutex { inner: UnsafeCell::new(0) }
        }

        #[inline]
        pub unsafe fn init(&mut self) {
            self.inner = UnsafeCell::new(0);
        }

        #[inline]
        pub unsafe fn lock(&self) {
            libnx::mutexLock(self.inner.get());
        }

        #[inline]
        pub unsafe fn unlock(&self) {
            libnx::mutexUnlock(self.inner.get());
        }

        #[inline]
        pub unsafe fn try_lock(&self) -> bool {
            libnx::mutexTryLock(self.inner.get()) 
        }

        #[inline]
        pub unsafe fn destroy(&self) {
        }
    }

    pub struct ReentrantMutex { inner: UnsafeCell<libnx::RMutex> }

    unsafe impl Send for ReentrantMutex {}
    unsafe impl Sync for ReentrantMutex {}

    impl ReentrantMutex {
        pub unsafe fn uninitialized() -> ReentrantMutex {
            ReentrantMutex { 
                inner: UnsafeCell::new(libnx::RMutex {
                    lock : 0,
                    thread_tag : 0,
                    counter : 0,
                })
            }
        }

        pub unsafe fn init(&mut self) {
            let mtx = libnx::RMutex {
                lock : 0,
                thread_tag : 0,
                counter : 0
            };
            self.inner = UnsafeCell::new(mtx);
        }

        pub unsafe fn lock(&self) {
            libnx::rmutexLock(self.inner.get());
        }

        #[inline]
        pub unsafe fn try_lock(&self) -> bool {
            libnx::rmutexTryLock(self.inner.get())
        }

        pub unsafe fn unlock(&self) {
            libnx::rmutexUnlock(self.inner.get());
        }

        pub unsafe fn destroy(&self) {}
    }

}

#[cfg(target_arch = "aarch64")]
pub use self::switch::*;

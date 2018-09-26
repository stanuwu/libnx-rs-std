// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[cfg(target_arch = "aarch64")]
pub use self::switch::*;
#[cfg(target_arch = "aarch64")]
mod switch {
    use mem;
    use cell::UnsafeCell;
    use libnx_rs::libnx;
    
    pub struct RWLock {
        inner : UnsafeCell<libnx::RwLock>
    }
    
    unsafe impl Send for RWLock {}
    unsafe impl Sync for RWLock {}

    impl RWLock {
        pub const fn new() -> RWLock {
            RWLock {
                inner : UnsafeCell::new(libnx::RwLock {
                    b : 0,
                    r : libnx::RMutex {
                        lock : 0,
                        thread_tag : 0,
                        counter : 0
                    },
                    g : libnx::RMutex {
                        lock : 0,
                        thread_tag : 0,
                        counter : 0
                    },
                })
            }

        }
        
        #[inline]
        pub unsafe fn read(&self) {
            libnx::rwlockReadLock(self.inner.get());
        }
        
        #[inline]
        pub unsafe fn write(&self) {
            libnx::rwlockWriteLock(self.inner.get());
        }
        
        #[inline]
        pub unsafe fn read_unlock(&self) {
            libnx::rwlockReadUnlock(self.inner.get());
        }

        #[inline]
        pub unsafe fn write_unlock(&self) {
            libnx::rwlockWriteUnlock(self.inner.get());
        }

        #[inline]
        pub unsafe fn try_read(&self) -> bool {
            let raw_ptr = &mut *self.inner.get();
            if !libnx::rmutexTryLock(&mut raw_ptr.r as *mut libnx::RMutex) {
                return false;
            }

            raw_ptr.b += 1;
            if raw_ptr.b == 0 {
                libnx::rmutexLock(&mut raw_ptr.g as *mut libnx::RMutex);
            }
            libnx::rmutexUnlock(&mut raw_ptr.r as *mut libnx::RMutex);
            true
        }
        
        #[inline]
        pub unsafe fn try_write(&self) -> bool {
            let raw_ptr = &mut *self.inner.get();
            libnx::rmutexTryLock(&mut raw_ptr.g as *mut libnx::RMutex)
        }

        #[inline]
        pub unsafe fn destroy(&self) {
            //TODO: this
        }

    } 
}

#[cfg(not(target_arch = "aarch64"))]
pub use self::nds::*;
#[cfg(not(target_arch = "aarch64"))]
mod nds {
    use cell::UnsafeCell;
    use super::mutex::Mutex;
    use super::condvar::Condvar;

    // A simple read-preferring RWLock implementation that I found on wikipedia <.<
    pub struct RWLock {
        mutex: Mutex,
        cvar: Condvar,
        reader_count: UnsafeCell<u32>, 
        writer_active: UnsafeCell<bool>,
    }

    unsafe impl Send for RWLock {}
    unsafe impl Sync for RWLock {}

    impl RWLock {
        pub const fn new() -> RWLock {
            RWLock {
                mutex: Mutex::new(),
                cvar: Condvar::new(),
                reader_count: UnsafeCell::new(0),
                writer_active: UnsafeCell::new(false),
            }
        }

        #[inline]
        pub unsafe fn read(&self) {
            self.mutex.lock();

            while *self.writer_active.get() {
                self.cvar.wait(&self.mutex);
            }

            assert!(*self.reader_count.get() != u32::max_value());
            *self.reader_count.get() += 1;

            self.mutex.unlock();
        }

        #[inline]
        pub unsafe fn try_read(&self) -> bool {
            if !self.mutex.try_lock() {
                return false
            }

            while *self.writer_active.get() {
                self.cvar.wait(&self.mutex);
            }

            assert!(*self.reader_count.get() != u32::max_value());
            *self.reader_count.get() += 1;

            self.mutex.unlock();
            true
        }

        #[inline]
        pub unsafe fn write(&self) {
            self.mutex.lock();

            while *self.writer_active.get() || *self.reader_count.get() > 0 {
                self.cvar.wait(&self.mutex);
            }

            *self.writer_active.get() = true;

            self.mutex.unlock();
        }

        #[inline]
        pub unsafe fn try_write(&self) -> bool {
            if !self.mutex.try_lock() {
                return false;
            }

            while *self.writer_active.get() || *self.reader_count.get() > 0 {
                self.cvar.wait(&self.mutex);
            }

            *self.writer_active.get() = true;

            self.mutex.unlock();
            true
        }

        #[inline]
        pub unsafe fn read_unlock(&self) {
            self.mutex.lock();

            *self.reader_count.get() -= 1;

            if *self.reader_count.get() == 0 {
                self.cvar.notify_one()
            }

            self.mutex.unlock();
        }

        #[inline]
        pub unsafe fn write_unlock(&self) {
            self.mutex.lock();

            *self.writer_active.get() = false;

            self.cvar.notify_all();

            self.mutex.unlock();
        }

        #[inline]
        pub unsafe fn destroy(&self) {
            self.mutex.destroy();
            self.cvar.destroy();
            *self.reader_count.get() = 0;
            *self.writer_active.get() = false;
        }
    }
}
use core::cell::RefMut;

use crate::{config::*, mm::PhysAddr, sync::UPSafeCell, task::PidHandle};
use alloc::{collections::BTreeMap, format, vec::Vec};
use lazy_static::lazy_static;

lazy_static! {
    static ref KS_MGR: UPSafeCell<KernelStackManager> = {
        log::info!(
            "need to construct KS_MGR, KernelStackManager struct size=0x{:x}",
            core::mem::size_of::<KernelStackManager>()
        );
        unsafe { UPSafeCell::new(KernelStackManager::new()) }
    };
}

// static mut KS_MGR: KernelStackManager = KernelStackManager::new();
fn get_ks_mgr() -> RefMut<'static, KernelStackManager> {
    KS_MGR.exclusive_access()
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    _data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    fn bottom(&self) -> usize {
        self as *const KernelStack as usize
    }
}

struct KernelStackManager {
    cursor: usize,
    recycled: Vec<usize>,
    record: BTreeMap<PidHandle, usize>, // pid => cursor
    stacks: [KernelStack; MAX_APP_NUM],
}

impl KernelStackManager {
    fn new() -> Self {
        KernelStackManager {
            cursor: 0,
            recycled: Vec::new(),
            record: BTreeMap::new(),
            stacks: [KernelStack {
                _data: [0; KERNEL_STACK_SIZE],
            }; MAX_APP_NUM],
        }
    }

    fn alloc_kernel_stack(&mut self, pid: PidHandle) -> (usize, usize) {
        let target = self.recycled.pop().unwrap_or_else(|| {
            let v = self.cursor;
            self.cursor += 1;
            v
        });
        self.record.insert(pid, target);
        let bottom = self.stacks[target].bottom();
        (bottom, bottom + KERNEL_STACK_SIZE)
    }

    fn lookup_kernel_stack(&mut self, pid: &PidHandle) -> (usize, usize) {
        let curosr = self
            .record
            .get(pid)
            .expect(&format!("lookup kernel stack by wrong pid? pid={}", pid.0));

        let bottom = self.stacks[*curosr].bottom();
        (bottom, bottom + KERNEL_STACK_SIZE)
    }
}

pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

fn get_appid_by_name(name: &str) -> Option<usize> {
    (0..get_num_app()).find(|&i| APP_NAMES[i] == name)
}

pub fn get_app_elf(name: &str) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    let app_id = get_appid_by_name(name).expect(&format!("wrong app name? name={}", name));
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}

pub fn list_apps() {
    println!("/**** APPS ****");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("**************/");
}

pub fn alloc_kernel_stack(pid: PidHandle) -> (usize, usize) {
    // unsafe { KS_MGR.alloc_kernel_stack(pid) }
    get_ks_mgr().alloc_kernel_stack(pid)
}

pub fn get_kernel_stack_top(pid: &PidHandle) -> PhysAddr {
    // let (_, stack_top) = unsafe { KS_MGR.lookup_kernel_stack(pid) };
    let (_, stack_top) = get_ks_mgr().lookup_kernel_stack(pid);
    PhysAddr::from(stack_top)
}

pub fn lookup_kernel_stack(pid: &PidHandle) -> (usize, usize) {
    // unsafe { KS_MGR.lookup_kernel_stack(pid) }
    get_ks_mgr().lookup_kernel_stack(pid)
}

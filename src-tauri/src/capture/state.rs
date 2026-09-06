use std::sync::Mutex;

// 元组结构体：唯一字段是 Mutex<Slot>。
// pub(super) = 只有父模块 capture 能直接碰里面的锁。
//
// 为什么包一层 Mutex？
// Tauri 会把 NativeMonitor 当全局状态，在多线程间共享；
// Mutex 保证同一时刻只有一个线程能改 Slot（拿/放 monitor）。
pub struct NativeMonitor(pub(super) Mutex<Slot>);

// 真正存数据的格子：macOS 上放一个 NSEvent monitor 句柄。
// Retained<AnyObject> ≈ ObjC 的强引用（ARC），指向 addLocalMonitor 返回的对象。
// Option：还没 start 时是 None；start 后 Some；stop 时 take 走再 remove。
pub struct Slot {
    #[cfg(target_os = "macos")] // 非 macOS 编译时这个字段直接消失，Slot 变成空结构体
    pub(super) monitor: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
}

// Send：类型的所有权可以挪到别的线程。
// Sync：类型可以通过 &T 被多线程同时读（这里实际靠外层 Mutex 串行化写入）。
//
// 编译器不会自动给 Slot 实现它们——里面的 ObjC Retained 句柄默认不是 Send/Sync。
// unsafe impl = 你向编译器承诺「在我们的用法里跨线程是安全的」。
// 安全靠两点：1) 外层 Mutex 互斥访问；2) 真正碰 NSEvent API 时走主线程（见 macos.rs）。
// 承诺错了编译器不管，会变成数据竞争 / UB。
unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

// 工厂：造一份空状态，交给 Tauri manage() 挂成全局。
// 初始 monitor = None，等 start() 再填进去。
pub fn managed() -> NativeMonitor {
    NativeMonitor(Mutex::new(Slot {
        #[cfg(target_os = "macos")]
        monitor: None,
    }))
}

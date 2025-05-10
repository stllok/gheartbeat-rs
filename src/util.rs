use crate::state::PID;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, Signal, System};

#[inline(always)]
pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .expect("SystemTime before UNIX EPOCH!!!")
}

#[inline(always)]
pub fn kill_process() -> ! {
    // 尝试发送SIGTERM信号，然后无论成功与否都退出进程
    let pid = Pid::from_u32(*PID);
    let system = System::new_all();
    
    if let Some(process) = system.process(pid) {
        println!("[gHeartbeat] SIGTERM request");
        let _ = process.kill_with(Signal::Term);
    }
    
    println!("[gHeartbeat] Process exit");
    std::process::exit(0);
}

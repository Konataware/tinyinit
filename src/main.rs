use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult, execvp, pause, getpgrp};
use nix::mount::{ MsFlags };
use std::ffi::CString;
use std::fs::create_dir_all;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use libc;

static CHILD_EXITED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigchld(_: i32) {
    // Reap all zombies!
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)){
            Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => {
                CHILD_EXITED.store(true, Ordering::Relaxed);
            }
            Ok(WaitStatus::StillAlive) => break,
            Ok(_) => break,
            Err(_) => break,
        }
    }
}

fn mount_filesystems() {
    // creates directories in case they don't exist
    let _ = create_dir_all("/dev");
    let _ = create_dir_all("/proc");
    let _ = create_dir_all("/sys");

    // mount devtmpfs (/dev/null, /dev/console/, /dev/tty [...])
    if let Err(e) = nix::mount::mount(
        Some("devtmpfs"),
        "/dev",
        Some("devtmpfs"),
        MsFlags::empty(),
        None::<&str>,
    ) {
        eprintln!("[ERROR] Failed to mount devtmpfs: {}", e);
    } else {
        println!("Mounted devtmpfs on /dev");
    }

    // mount proc (ps, top, [...])
    if let Err(e) = nix::mount::mount(
        Some("/proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    ) {
        eprintln!("Failed to mount proc: {}", e);
    }

    // mounts sysfs
    let _ = nix::mount::mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None::<&str>
    );
}

fn setup_signal_handler() -> Result<(), nix::Error> {
    let handler = SigHandler::Handler(handle_sigchld);
    let sig_action = SigAction::new(
        handler,
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    unsafe { sigaction(Signal::SIGCHLD, &sig_action) }?;
    Ok(())
}

fn main() {
    if let Err(e) = setup_signal_handler() {
        eprintln!("[ERROR] Failed to setup signal handler: {}", e);
        exit(1);
    }

    mount_filesystems();

    // ignoring SIGTTOU so the parent doesn't get stopped when child calls tcsetpgrp
    let ignore_action = SigAction::new(
        SigHandler::SigIgn,
        SaFlags::empty(),
        SigSet::empty(),
    );
    unsafe {
        let _ = sigaction(Signal::SIGTTOU, &ignore_action);
    }

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child: _child_pid }) => {
            // parent: wait for signals only, no terminal handling
            loop {
                pause();
                if CHILD_EXITED.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
        #[allow(unused_unsafe)]
        #[allow(unreachable_code)]
        Ok(ForkResult::Child) => {
            // turns into session leader
            unsafe { libc::setsid(); }

            // makes the process group be the foreground
            let stdin_fd = 0;
            let result = unsafe { libc::tcsetpgrp(stdin_fd, getpgrp().as_raw()) };
            if result != 0 {
                let errno = nix::errno::Errno::last_raw();
                if errno != libc::ENOTTY {
                    eprintln!("[ERROR] tcsetpgrp failed: {}", errno);
                }
            }

            let shell = CString::new("/busybox").unwrap();
            let arg1 = CString::new("sh").unwrap();
            let arg2 = CString::new("-i").unwrap();
            let args = &[shell.as_c_str(), arg1.as_c_str(), arg2.as_c_str()];
            execvp(&shell, args).expect("execvp failed");
        }
        Err(e) => {
            eprintln!("[ERROR] Fork failed: {}", e);
        }
    }
}
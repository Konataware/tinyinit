use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal, sigprocmask, SigmaskHow};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult, execvp, pause, setpgid, getpgrp, Pid};
use nix::errno::Errno;
use std::ffi::CString;
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

    // ignoring SIGTTOU so the parent doesn't get stopped when child calls tcsetpgrp
    let ignore_action = SigAction::new(
        SigHandler::SigIgn,
        SaFlags::empty(),
        SigSet::empty(),
    );
    unsafe {
        let _ = sigaction(Signal::SIGTTOU, &ignore_action);
        let _ = sigaction(Signal::SIGTTIN, &ignore_action);
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

            unsafe { libc::setsid(); }
            // places child into its own process group
            setpgid(Pid::from_raw(0), Pid::from_raw(0)).expect("setpgid failed");

            let stdin_fd = 0;
            unsafe { libc::ioctl(stdin_fd, libc::TIOCSCTTY, 0); }

            let mut new_sigset = SigSet::empty();
            new_sigset.add(Signal::SIGTTOU);
            let mut old_sigset = SigSet::empty();
            unsafe {
                sigprocmask(SigmaskHow::SIG_BLOCK, Some(&new_sigset), Some(&mut old_sigset))
                    .expect("[ERROR] sigprocmask block failed!");
            }

            // make this process group the foreground of the controlling terminal
            let result = unsafe { libc::tcsetpgrp(stdin_fd, getpgrp().as_raw()) };
            if result != 0 {
                let errno = Errno::last_raw();
                if errno != libc::ENOTTY {
                    eprintln!("[ERROR] tcsetpgrp failed: {}", errno);
                }
            }

            unsafe {
                sigprocmask(SigmaskHow::SIG_SETMASK, Some(&old_sigset), None)
                    .expect("[ERROR] sigprocmask restore failed");
            }

            // executes the shell
            let shell = CString::new("/bin/bash").unwrap();
            let dash_i = CString::new("-i").unwrap();
            let args = &[shell.as_c_str(), dash_i.as_c_str()];
            execvp(&shell, args).expect("execvp failed");
        }
        Err(e) => {
            eprintln!("[ERROR] Fork failed: {}", e);
        }
    }
}
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult, execvp, pause, setpgid, getpgrp, Pid};
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
                /*
                A child exited, could be a main one or a zombie. 
                Ideally we check PID for this. For minimum viable simplicity, we just mark it as exited.
                */
                CHILD_EXITED.store(true, Ordering::Relaxed);
                // Continues loop to kill any other children
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

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child: _child_pid }) => {
            // Parent, wait for signals.
            loop {
                pause();
                if CHILD_EXITED.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
        #[allow(unreachable_code)]
        Ok(ForkResult::Child) => {
            // Puts child in its own process group
            setpgid(Pid::from_raw(0), Pid::from_raw(0)).expect("setpgid");
            
            // Make it the foreground process group of the terminal
            let stdin_fd = 0; // File descriptor 0 is stdin
            let result = unsafe { libc::tcsetpgrp(stdin_fd, getpgrp().as_raw()) };
            if result == 0 {
                // Success, nothing to do
            } else {
                let errno = nix::errno::Errno::last_raw(); // returns i32
                if errno == libc::ENOTTY {
                    // NO TTY - WE CONTINUE
                } else {
                    eprintln!("[ERROR] tcsetpgrp failed: {}", errno);
                }
            }

            let shell = CString::new("/bin/sh").unwrap();
            let args = &[shell.as_c_str()];
            execvp(&shell, args).expect("execvp failed");
            
        }
        Err(e) => { println!("[ERROR] Fork failed: {}", e) }
    } 
}
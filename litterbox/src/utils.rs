use log::trace;
use std::process::Command;

pub fn trace_arguments(cmd: &Command) {
    trace!(
        "Will run: {} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args().fold(String::new(), |mut acc, arg| {
            acc.push_str(&arg.to_string_lossy());
            acc.push(' ');
            acc
        })
    );
}

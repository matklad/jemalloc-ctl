//! Bulk statistics output.

use std::any::Any;
use std::ffi::CStr;
use std::io::{self, Write};
use std::os::raw::{c_char, c_void};
use std::panic::{self, AssertUnwindSafe};

use malloc_stats_print;

/// Statistics configuration.
///
/// All options default to `false`.
#[derive(Copy, Clone, Default)]
pub struct Options {
    /// If set, information that never changes during execution will be skipped.
    ///
    /// This corresponds to the `g` character.
    pub skip_constant: bool,

    /// If set, merged information about arenas will be skipped.
    ///
    /// This corresponds to the `m` character.
    pub skip_merged_arena: bool,

    /// If set, information about individual arenas will be skipped.
    ///
    /// This corresponds to the `a` character.
    pub skip_per_arena: bool,

    /// If set, information about individual size classes for bins will be skipped.
    ///
    /// This corresponds to the `b` character.
    pub skip_bin_size_classes: bool,

    /// If set, information about individual size classes for large objects will be skipped.
    ///
    /// This corresponds to the `l` character.
    pub skip_large_size_classes: bool,

    _p: (),
}

struct State<W> {
    writer: W,
    error: io::Result<()>,
    panic: Result<(), Box<Any + Send>>,
}

unsafe extern "C" fn callback<W>(opaque: *mut c_void, buf: *const c_char)
where
    W: Write,
{
    let state = &mut *(opaque as *mut State<W>);
    if state.error.is_err() || state.panic.is_err() {
        return;
    }

    let buf = CStr::from_ptr(buf);
    match panic::catch_unwind(AssertUnwindSafe(|| state.writer.write(buf.to_bytes()))) {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => state.error = Err(e),
        Err(e) => state.panic = Err(e),
    }
}

/// Writes allocator statistics.
///
/// The information is the same that can be retrieved by the individual lookup methods in this
/// crate, but all done at once.
pub fn stats_print<W>(writer: W, options: Options) -> io::Result<()>
where
    W: Write,
{
    unsafe {
        let mut state = State {
            writer,
            error: Ok(()),
            panic: Ok(()),
        };
        let mut opts = [0; 6];
        let mut i = 0;
        if options.skip_constant {
            opts[i] = b'g' as c_char;
            i += 1;
        }
        if options.skip_merged_arena {
            opts[i] = b'm' as c_char;
            i += 1;
        }
        if options.skip_per_arena {
            opts[i] = b'a' as c_char;
            i += 1;
        }
        if options.skip_bin_size_classes {
            opts[i] = b'b' as c_char;
            i += 1;
        }
        if options.skip_large_size_classes {
            opts[i] = b'l' as c_char;
            i += 1;
        }
        opts[i] = 0;

        malloc_stats_print(Some(callback::<W>), &mut state as *mut _ as *mut c_void, opts.as_ptr());
        if let Err(e) = state.panic {
            panic::resume_unwind(e);
        }
        state.error
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic() {
        let mut buf = vec![];
        stats_print(&mut buf, Options::default()).unwrap();
        println!("{}", String::from_utf8(buf).unwrap());
    }
}

macro_rules! eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

#[cfg(not(feature = "host"))]
mod device_app;
#[cfg(feature = "host")]
mod host_app;

fn main() {
    if cfg!(feature = "stock-tg5040") {
        // Stock permits only the reviewed foreground Apps entry, not maintenance CLI modes.
        let refusal = if std::env::args_os().len() != 1 {
            Err(std::io::Error::other(
                "stock maintenance/extra arguments refused",
            ))
        } else {
            slot::stock_tg5040::prepare()
        };
        if let Err(error) = refusal {
            eprintln!("slot: stock safety check: {error}; no hardware has been initialized");
            std::process::exit(slot::stock_tg5040::BLOCKED_EXIT);
        }
    }
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("--install-bootlogo") {
        let Some(card) = std::env::var_os("SLOT_ROOT") else {
            eprintln!("slot: --install-bootlogo needs SLOT_ROOT");
            std::process::exit(2);
        };
        let card = std::path::Path::new(&card);
        slot::bootlogo::refresh(card);
        slot::bootlogo::remember(card);
        return;
    }
    if first.as_deref() == Some("--init-root") {
        let Some(dir) = args.next() else {
            eprintln!("slot: --init-root needs a directory");
            std::process::exit(2);
        };
        slot::root::ensure(std::path::Path::new(&dir));
        return;
    }
    slot::crash::install();

    #[cfg(feature = "host")]
    host_app::run();

    #[cfg(not(feature = "host"))]
    device_app::run();
}

//! `sage-receipt-verify <envelope.json> [--labels <labels.jsonl>]`
//!
//! Verifies one signed SAGE outcome receipt and prints the verdict as JSON. With `--labels`, a valid
//! receipt's label is appended to the file as one JSON line, once: a digest already in the file is
//! not written again. Exit code 0 = valid, 1 = invalid, 2 = usage or I/O error. Off-consensus by
//! construction: this writes a file a researcher reads, nothing the node reads.

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, labels) = match args.as_slice() {
        [p] => (p.clone(), None),
        [p, flag, l] if flag == "--labels" => (p.clone(), Some(l.clone())),
        _ => {
            eprintln!("usage: sage-receipt-verify <envelope.json> [--labels <labels.jsonl>]");
            return ExitCode::from(2);
        }
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    match sage_receipts::verify_to_label(&data) {
        Err(e) => {
            println!("{}", serde_json::json!({ "valid": false, "error": e.0 }));
            ExitCode::from(1)
        }
        Ok(label) => {
            let line = serde_json::to_string(&label).expect("a label always serializes");
            if let Some(l) = labels {
                let existing = std::fs::read_to_string(&l).unwrap_or_default();
                let already = existing
                    .lines()
                    .any(|x| x.contains(&format!("\"receipt_digest\":\"{}\"", label.receipt_digest)));
                if !already {
                    let written = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&l)
                        .and_then(|mut f| writeln!(f, "{line}"));
                    if let Err(e) = written {
                        eprintln!("append {l}: {e}");
                        return ExitCode::from(2);
                    }
                }
            }
            println!(
                "{}",
                serde_json::json!({ "valid": true, "receipt_digest": label.receipt_digest, "label": label })
            );
            ExitCode::SUCCESS
        }
    }
}

//! Batches raw pty output so the frontend gets few, larger messages: a batch is flushed
//! ~10 ms after its first byte or once it reaches 64 KB. Bytes are passed through
//! untouched; UTF-8 is decoded (and re-joined across batches) by the frontend.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::model::{ExitInfo, PtyOutput};

pub const FLUSH_INTERVAL: Duration = Duration::from_millis(10);
pub const MAX_BATCH: usize = 64 * 1024;
/// After the child exits, output still in the pty buffer may arrive for a moment.
pub const EXIT_GRACE: Duration = Duration::from_millis(250);

/// Messages from the reader and waiter threads.
#[derive(Debug)]
pub enum PumpMsg {
    Data(Vec<u8>),
    Exited(ExitInfo),
}

/// Runs until the reader hit EOF after the exit, or the exit grace period ran out.
/// Always ends with exactly one `PtyOutput::Exit`.
pub fn run_pump(rx: &Receiver<PumpMsg>, mut sink: impl FnMut(PtyOutput)) {
    let mut batch: Vec<u8> = Vec::new();
    let mut flush_at: Option<Instant> = None;
    let mut exit: Option<ExitInfo> = None;
    let mut exit_deadline: Option<Instant> = None;

    loop {
        let deadline = match (flush_at, exit_deadline) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let received = match deadline {
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
        };

        match received {
            Ok(PumpMsg::Data(chunk)) => {
                if batch.is_empty() {
                    flush_at = Some(Instant::now() + FLUSH_INTERVAL);
                }
                batch.extend_from_slice(&chunk);
                if batch.len() >= MAX_BATCH {
                    flush(&mut batch, &mut flush_at, &mut sink);
                }
            }
            Ok(PumpMsg::Exited(info)) => {
                exit = Some(info);
                exit_deadline = Some(Instant::now() + EXIT_GRACE);
            }
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                if flush_at.is_some_and(|at| now >= at) {
                    flush(&mut batch, &mut flush_at, &mut sink);
                }
                if exit_deadline.is_some_and(|at| now >= at) {
                    break;
                }
            }
            // Both the reader (EOF) and the waiter (exit sent) are done.
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    flush(&mut batch, &mut flush_at, &mut sink);
    sink(PtyOutput::Exit(exit.unwrap_or(ExitInfo {
        code: None,
        signal: None,
    })));
}

fn flush(batch: &mut Vec<u8>, flush_at: &mut Option<Instant>, sink: &mut impl FnMut(PtyOutput)) {
    *flush_at = None;
    if !batch.is_empty() {
        sink(PtyOutput::Data(std::mem::take(batch)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn exit0() -> ExitInfo {
        ExitInfo {
            code: Some(0),
            signal: None,
        }
    }

    fn collect(messages: Vec<PumpMsg>) -> Vec<PtyOutput> {
        let (tx, rx) = mpsc::channel();
        for message in messages {
            tx.send(message).expect("send");
        }
        drop(tx);
        let mut out = Vec::new();
        run_pump(&rx, |o| out.push(o));
        out
    }

    #[test]
    fn coalesces_small_chunks_into_one_batch() {
        let chunks = (0..100).map(|_| PumpMsg::Data(b"ab".to_vec()));
        let out = collect(chunks.chain([PumpMsg::Exited(exit0())]).collect());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], PtyOutput::Data(b"ab".repeat(100)));
        assert_eq!(out[1], PtyOutput::Exit(exit0()));
    }

    #[test]
    fn flushes_at_the_size_limit() {
        let big = vec![b'x'; 40 * 1024];
        let out = collect(vec![
            PumpMsg::Data(big.clone()),
            PumpMsg::Data(big.clone()),
            PumpMsg::Data(big),
            PumpMsg::Exited(exit0()),
        ]);
        let sizes: Vec<usize> = out
            .iter()
            .filter_map(|o| match o {
                PtyOutput::Data(d) => Some(d.len()),
                PtyOutput::Exit(_) => None,
            })
            .collect();
        assert_eq!(sizes, [80 * 1024, 40 * 1024]);
    }

    #[test]
    fn flushes_after_the_interval_without_more_data() {
        let (tx, rx) = mpsc::channel();
        let pump = std::thread::spawn(move || {
            let mut out = Vec::new();
            run_pump(&rx, |o| out.push((Instant::now(), o)));
            out
        });
        tx.send(PumpMsg::Data(b"hi".to_vec())).expect("send");
        // The interval alone must flush: the data is out before anything else arrives. (Not a
        // tight deadline: with ~250 tests in parallel the pump thread can be scheduled late.)
        std::thread::sleep(FLUSH_INTERVAL * 50);
        let exit_sent = Instant::now();
        tx.send(PumpMsg::Exited(exit0())).expect("send");
        drop(tx);
        let out = pump.join().expect("join");
        let (at, first) = &out[0];
        assert_eq!(*first, PtyOutput::Data(b"hi".to_vec()));
        assert!(
            *at < exit_sent,
            "flushed only by the exit, not by the interval"
        );
    }

    #[test]
    fn keeps_output_that_arrives_after_the_exit() {
        let out = collect(vec![
            PumpMsg::Exited(exit0()),
            PumpMsg::Data(b"late".to_vec()),
        ]);
        assert_eq!(out[0], PtyOutput::Data(b"late".to_vec()));
        assert_eq!(out[1], PtyOutput::Exit(exit0()));
    }

    #[test]
    fn exit_grace_bounds_the_wait() {
        let (tx, rx) = mpsc::channel();
        tx.send(PumpMsg::Exited(exit0())).expect("send");
        let started = Instant::now();
        let mut out = Vec::new();
        run_pump(&rx, |o| out.push(o)); // `tx` is still alive: only the grace period ends this
        assert!(started.elapsed() >= EXIT_GRACE);
        assert_eq!(out, [PtyOutput::Exit(exit0())]);
        drop(tx);
    }
}

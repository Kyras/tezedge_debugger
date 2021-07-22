// Copyright (c) SimpleStaking, Viable Systems and Tezedge Contributors
// SPDX-License-Identifier: MIT

use std::ops::Deref;
use std::sync::{Arc, Mutex, atomic::{Ordering, AtomicU32}};
use bpf_memprof_common::{EventKind, Event};
use super::{Reporter, StackResolver, FrameReport, aggregator::Aggregator};

impl Reporter for Aggregator {
    fn short_report(&self) -> (u64, u64) {
        let (mut value, mut cache_value) = (0, 0);
        for (v, c, _) in self.report() {
            value += v;
            cache_value += c;
        }

        (value, cache_value)
    }

    fn tree_report<R>(&self, resolver: R, threshold: u64, reverse: bool) -> FrameReport<R>
    where
        R: Deref<Target = StackResolver>,
    {
        let mut report = FrameReport::new(resolver);
        for (value, cache_value, stack) in self.report() {
            if reverse {
                report.inner.insert(stack.iter().rev(), value, cache_value);
            } else {
                report.inner.insert(stack.iter(), value, cache_value);
            }
        }
        report.inner.strip(threshold);

        report

    }
}

pub struct Consumer {
    has_pid: bool,
    pid: Arc<AtomicU32>,
    aggregator: Arc<Mutex<Aggregator>>,
    last: Option<EventKind>,
    limit: Arc<AtomicU32>,
    killed: bool,
}

impl Consumer {
    pub fn reporter(&self) -> Arc<Mutex<Aggregator>> {
        self.aggregator.clone()
    }

    pub fn pid(&self) -> Arc<AtomicU32> {
        self.pid.clone()
    }
}

impl Consumer {
    pub fn new(limit: Arc<AtomicU32>) -> Self {
        Consumer {
            has_pid: false,
            pid: Arc::new(AtomicU32::new(0)),
            aggregator: Arc::new(Mutex::new(Aggregator::default())),
            last: None,
            limit,
            killed: false,
        }
    }

    pub fn arrive(&mut self, data: &[u8]) {
        if self.killed {
            return;
        }

        let event = match Event::from_slice(data) {
            Ok(v) => v,
            Err(error) => {
                log::error!("failed to read slice from kernel: {}", error);
                return;
            }
        };

        if let Some(last) = &self.last {
            if last.eq(&event.event) {
                log::trace!("repeat");
                return;
            }
        }
        match &event.event {
            &EventKind::PageAlloc(ref v) if v.pfn.0 != 0 => {
                self.has_pid = true;
                self.pid.store(event.pid, Ordering::SeqCst);
                self.aggregator.lock().unwrap().track_alloc(v.pfn.0 as u32, v.order as u8, &event.stack);
            }
            &EventKind::PageFree(ref v) if v.pfn.0 != 0 && self.has_pid => {
                self.aggregator.lock().unwrap().track_free(v.pfn.0 as u32);
            },
            &EventKind::AddToPageCache(ref v) if v.pfn.0 != 0 && self.has_pid => {
                self.aggregator.lock().unwrap().mark_cache(v.pfn.0 as u32, true);
            },
            &EventKind::RemoveFromPageCache(ref v) if v.pfn.0 != 0 && self.has_pid => {
                self.aggregator.lock().unwrap().mark_cache(v.pfn.0 as u32, false);
            },
            &EventKind::RssStat(ref v) if v.member == 1 && self.has_pid => {
                use nix::{unistd::Pid, sys::signal};

                let limit = self.limit.load(Ordering::Relaxed);
                if limit != 0 && (v.size / 1000) as u32 > limit {
                    let pid = Pid::from_raw(self.pid.load(Ordering::Relaxed) as _);
                    match signal::kill(pid, signal::SIGKILL) {
                        Ok(()) => (),
                        Err(e) => log::error!("cannot kill pid: {}, errno: {}", pid, e),
                    }
                    self.killed = true;
                }
                self.aggregator.lock().unwrap().track_rss_anon(v.size as _);
            }
            _ => (),
        }
        self.last = Some(event.event);
    }
}

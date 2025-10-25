//! RO:WHAT — Mailbox<T>: bounded queue facade around tokio mpsc, with split().
//! RO:WHY  — Single-consumer FIFO; Busy on overflow; per-message deadlines; explicit close via drop.
//! RO:INTERACTS — mailbox::observer hooks, errors.
//! RO:INVARIANTS — try_send rejects when full; pull uses timeout(deadline). No PII logged.

#![forbid(unsafe_code)]

use super::error::{MailboxError, MailboxResult};
use super::observer::{DropReason, Observer};
use crate::observe::trace;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::{SendError, TrySendError};

/// Factory wrapper that owns both ends until you call split().
pub struct Mailbox<T> {
    actor: String,
    tx: mpsc::Sender<T>,
    rx: mpsc::Receiver<T>,
    capacity: usize,
    max_msg_bytes: usize,
    deadline: Duration,
    observer: Observer,
}

impl<T> Mailbox<T> {
    pub(crate) fn new(
        actor: String,
        capacity: usize,
        max_msg_bytes: usize,
        deadline: Duration,
        observer: Observer,
    ) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self {
            actor,
            tx,
            rx,
            capacity,
            max_msg_bytes,
            deadline,
            observer,
        }
    }

    /// Non-blocking enqueue; returns Busy when full.
    pub fn try_send(&self, msg: T) -> MailboxResult<()> {
        match self.tx.try_send(msg) {
            Ok(()) => {
                // Depth is not exposed by tokio mpsc; still notify enqueue for hooks.
                self.observer.on_enqueue(&self.actor, 0);
                trace::span_enqueue(&self.actor, 0);
                Ok(())
            }
            Err(TrySendError::Full(_msg)) => {
                self.observer.on_drop(&self.actor, DropReason::Capacity);
                Err(MailboxError::Busy)
            }
            Err(TrySendError::Closed(_msg)) => Err(MailboxError::Closed),
        }
    }

    /// Blocking send with the mailbox’s per-message deadline (timeout).
    pub async fn send(&self, msg: T) -> MailboxResult<()> {
        tokio::select! {
            biased;
            _ = tokio::time::sleep(self.deadline) => {
                self.observer.on_timeout(&self.actor);
                trace::span_handle(&self.actor, "timeout", self.deadline.as_millis() as u64);
                Err(MailboxError::Timeout)
            }
            res = self.tx.send(msg) => {
                match res {
                    Ok(()) => Ok(()),
                    Err(SendError(_msg)) => Err(MailboxError::Closed),
                }
            }
        }
    }

    /// Pull one message, honoring the deadline as a receive timeout.
    pub async fn pull(&mut self) -> MailboxResult<T> {
        match tokio::time::timeout(self.deadline, self.rx.recv()).await {
            Ok(Some(m)) => Ok(m),
            Ok(None) => Err(MailboxError::Closed),
            Err(_) => {
                self.observer.on_timeout(&self.actor);
                trace::span_handle(&self.actor, "timeout", self.deadline.as_millis() as u64);
                Err(MailboxError::Timeout)
            }
        }
    }

    /// Configured bounded capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Deadline getter.
    pub fn deadline(&self) -> Duration {
        self.deadline
    }

    /// Split into a producer (tx) and consumer (rx). Dropping tx will *close*
    /// the channel, ensuring drains can terminate (recv => None).
    pub fn split(self) -> (MailboxTx<T>, MailboxRx<T>) {
        let Mailbox {
            actor,
            tx,
            rx,
            capacity: _,
            max_msg_bytes: _,
            deadline,
            observer,
        } = self;
        (
            MailboxTx {
                actor: actor.clone(),
                tx,
                observer: observer.clone(),
            },
            MailboxRx {
                actor,
                rx,
                deadline,
                observer,
            },
        )
    }
}

/// Send half of a mailbox.
pub struct MailboxTx<T> {
    actor: String,
    tx: mpsc::Sender<T>,
    observer: Observer,
}

impl<T> MailboxTx<T> {
    pub fn try_send(&self, msg: T) -> MailboxResult<()> {
        match self.tx.try_send(msg) {
            Ok(()) => {
                self.observer.on_enqueue(&self.actor, 0);
                trace::span_enqueue(&self.actor, 0);
                Ok(())
            }
            Err(TrySendError::Full(_msg)) => {
                self.observer.on_drop(&self.actor, DropReason::Capacity);
                Err(MailboxError::Busy)
            }
            Err(TrySendError::Closed(_msg)) => Err(MailboxError::Closed),
        }
    }

    pub async fn send(&self, msg: T) -> MailboxResult<()> {
        match self.tx.send(msg).await {
            Ok(()) => Ok(()),
            Err(SendError(_msg)) => Err(MailboxError::Closed),
        }
    }
}

/// Receive half of a mailbox.
pub struct MailboxRx<T> {
    actor: String,
    rx: mpsc::Receiver<T>,
    deadline: Duration,
    observer: Observer,
}

impl<T> MailboxRx<T> {
    pub async fn pull(&mut self) -> MailboxResult<T> {
        match tokio::time::timeout(self.deadline, self.rx.recv()).await {
            Ok(Some(m)) => Ok(m),
            Ok(None) => Err(MailboxError::Closed),
            Err(_) => {
                self.observer.on_timeout(&self.actor);
                trace::span_handle(&self.actor, "timeout", self.deadline.as_millis() as u64);
                Err(MailboxError::Timeout)
            }
        }
    }

    pub fn deadline(&self) -> Duration {
        self.deadline
    }
}

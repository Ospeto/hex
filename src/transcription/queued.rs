//! A bounded output queue owns this deferred work before it may start.
//! Dropping or rejecting a job must not perform its paid network request.

pub(crate) struct Queued<T>(Box<dyn FnOnce() -> T + Send>);

impl<T> Queued<T> {
    pub(crate) fn new(work: impl FnOnce() -> T + Send + 'static) -> Self {
        Self(Box::new(work))
    }

    /// Called only by the worker after receiving an admitted output job.
    pub(crate) fn wait(self) -> T {
        (self.0)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };

    fn request(starts: &Arc<AtomicUsize>) -> Queued<()> {
        let starts = starts.clone();
        Queued::new(move || {
            starts.fetch_add(1, Ordering::SeqCst);
        })
    }

    #[test]
    fn full_and_disconnected_output_queues_never_start_rejected_requests() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .try_send(request(&starts))
            .unwrap_or_else(|_| panic!("empty queue"));
        let rejected = sender.try_send(request(&starts));
        assert!(matches!(rejected, Err(mpsc::TrySendError::Full(_))));
        drop(rejected);
        drop(receiver);
        let rejected = sender.try_send(request(&starts));
        assert!(matches!(rejected, Err(mpsc::TrySendError::Disconnected(_))));
        drop(rejected);
        assert_eq!(starts.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn admitted_requests_start_only_when_the_worker_waits() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .try_send(request(&starts))
            .unwrap_or_else(|_| panic!("empty queue"));
        let job = receiver.recv().unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        job.wait();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_job_discarded_by_its_worker_never_starts() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .try_send(request(&starts))
            .unwrap_or_else(|_| panic!("empty queue"));
        // As when the output worker observes cancellation/shutdown after recv.
        drop(receiver.recv().unwrap());
        assert_eq!(starts.load(Ordering::SeqCst), 0);
    }
}

//! Optimized Tokio worker pools for high concurrency

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Worker handle (optimized)
type WorkerHandle = tokio::task::JoinHandle<()>;

/// Dynamic worker pool that auto-scales based on load
pub struct DynamicWorkerPool {
    workers: Arc<Mutex<HashMap<String, WorkerHandle>>>,
    max_workers: usize,
    min_workers: usize,
    active_tasks: Arc<AtomicUsize>,
    queue: Arc<crossbeam::queue::SegQueue<tokio::task::JoinHandle<()>>>,
}

impl DynamicWorkerPool {
    pub fn new(min_workers: usize, max_workers: usize) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            min_workers,
            max_workers,
            active_tasks: Arc::new(AtomicUsize::new(0)),
            queue: Arc::new(crossbeam::queue::SegQueue::new()),
        }
    }

    /// Submit work to the pool (optimized)
    pub async fn submit<F, Fut>(&self, worker_id: &str, work: F) -> Result<(), WorkerPoolError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let workers = self.workers.lock();
        if workers.contains_key(worker_id) {
            drop(workers);

            // Increment active task counter
            self.active_tasks.fetch_add(1, Ordering::SeqCst);

            let active_tasks = Arc::clone(&self.active_tasks);

            // Spawn optimized task
            let handle = tokio::spawn(async move {
                work().await;
                active_tasks.fetch_sub(1, Ordering::SeqCst);
            });

            self.queue.push(handle);
            Ok(())
        } else {
            Err(WorkerPoolError::WorkerNotFound)
        }
    }

    /// Get current load metric
    pub fn current_load(&self) -> f64 {
        let active = self.active_tasks.load(Ordering::SeqCst);
        let total = self.workers.lock().len().max(1);
        active as f64 / total as f64
    }

    /// Auto-scale workers based on load
    pub fn auto_scale(&self) {
        let load = self.current_load();
        let mut workers = self.workers.lock();

        if load > 0.8 && workers.len() < self.max_workers {
            let new_worker_id = format!("worker-{}", workers.len());
            // Spawn new worker (simplified)
            let handle = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            });
            workers.insert(new_worker_id, handle);
        } else if load < 0.3 && workers.len() > self.min_workers {
            // Remove excess worker (simplified)
            if let Some(key) = workers.keys().next().cloned() {
                if let Some(handle) = workers.remove(&key) {
                    handle.abort();
                }
            }
        }
    }
}

impl Drop for DynamicWorkerPool {
    fn drop(&mut self) {
        // Graceful shutdown of all workers
        let workers = self.workers.lock();
        for handle in workers.values() {
            handle.abort();
        }
    }
}

/// Optimized Circuit breaker for resilience
pub struct CircuitBreaker {
    state: Arc<parking_lot::Mutex<CircuitBreakerState>>,
    threshold: u32,
    timeout: std::time::Duration,
    failures: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
    LastFailure(std::time::Instant),
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout: std::time::Duration) -> Self {
        Self {
            state: Arc::new(parking_lot::Mutex::new(CircuitBreakerState::Closed)),
            threshold,
            timeout,
            failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>
            + Send,
    {
        let state = self.state.lock();

        match *state {
            CircuitBreakerState::Open => {
                drop(state);
                Err(CircuitBreakerError::Open)
            }
            CircuitBreakerState::HalfOpen => {
                drop(state);
                match operation().await {
                    Ok(result) => {
                        self.reset();
                        Ok(result)
                    }
                    Err(_) => {
                        self.record_failure();
                        Err(CircuitBreakerError::ExecutionFailed)
                    }
                }
            }
            _ => {
                drop(state);
                match operation().await {
                    Ok(result) => {
                        self.reset();
                        Ok(result)
                    }
                    Err(_) => {
                        self.record_failure();
                        Err(CircuitBreakerError::ExecutionFailed)
                    }
                }
            }
        }
    }

    fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= self.threshold as usize {
            let mut state = self.state.lock();
            *state = CircuitBreakerState::Open;
        }
    }

    fn reset(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut state = self.state.lock();
        *state = CircuitBreakerState::Closed;
    }

    pub fn half_open(&self) {
        let mut state = self.state.lock();
        *state = CircuitBreakerState::HalfOpen;
    }
}

/// Optimized Backpressure controller
pub struct BackpressureController {
    max_queue_size: usize,
    current_queue_size: Arc<AtomicUsize>,
    rejection_count: Arc<AtomicUsize>,
}

impl BackpressureController {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            max_queue_size,
            current_queue_size: Arc::new(AtomicUsize::new(0)),
            rejection_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn can_accept(&self) -> bool {
        let size = self.current_queue_size.load(Ordering::SeqCst);
        size < self.max_queue_size
    }

    pub fn record_enqueue(&self) -> Result<(), BackpressureError> {
        let size = self.current_queue_size.fetch_add(1, Ordering::SeqCst) + 1;
        if size > self.max_queue_size {
            self.current_queue_size.fetch_sub(1, Ordering::SeqCst);
            self.rejection_count.fetch_add(1, Ordering::SeqCst);
            return Err(BackpressureError::QueueFull);
        }
        Ok(())
    }

    pub fn record_dequeue(&self) {
        self.current_queue_size.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn rejection_rate(&self) -> f64 {
        let rejections = self.rejection_count.load(Ordering::SeqCst) as f64;
        let total_attempts = rejections + self.current_queue_size.load(Ordering::SeqCst) as f64;
        if total_attempts > 0.0 {
            rejections / total_attempts
        } else {
            0.0
        }
    }
}

/// Rate limiter with token bucket
pub struct RateLimiter {
    capacity: usize,
    tokens: Arc<AtomicUsize>,
    refill_rate: std::time::Duration,
    last_refill: Arc<Mutex<std::time::Instant>>,
}

impl RateLimiter {
    pub fn new(capacity: usize, refill_rate: std::time::Duration) -> Self {
        let tokens = Arc::new(AtomicUsize::new(capacity));
        let last_refill = Arc::new(Mutex::new(std::time::Instant::now()));

        // Start refill task
        let tokens_ref = Arc::clone(&tokens);
        let last_refill_ref = Arc::clone(&last_refill);
        let refill_rate_clone = refill_rate;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refill_rate_clone);
            loop {
                interval.tick().await;
                let mut last = last_refill_ref.lock();
                *last = std::time::Instant::now();
                tokens_ref.store(capacity, Ordering::SeqCst);
            }
        });

        Self {
            capacity,
            tokens,
            refill_rate,
            last_refill,
        }
    }

    pub async fn acquire(&self) -> Result<(), RateLimitError> {
        let current = self.tokens.fetch_sub(1, Ordering::SeqCst);
        if current > 0 {
            Ok(())
        } else {
            self.tokens.fetch_add(1, Ordering::SeqCst); // Restore token
            Err(RateLimitError::RateExceeded)
        }
    }

    pub fn remaining_tokens(&self) -> usize {
        self.tokens.load(Ordering::SeqCst)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkerPoolError {
    #[error("worker not found")]
    WorkerNotFound,

    #[error("pool at capacity")]
    AtCapacity,
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("circuit breaker is open")]
    Open,

    #[error("execution failed")]
    ExecutionFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum BackpressureError {
    #[error("queue is full")]
    QueueFull,
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded")]
    RateExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_pool_load() {
        let pool = DynamicWorkerPool::new(1, 5);

        // Check load calculation with no active tasks
        let load = pool.current_load();
        assert_eq!(load, 0.0);

        // Test auto-scaling
        pool.auto_scale();
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(2, std::time::Duration::from_secs(1));

        // First failure
        assert!(
            breaker
                .execute(|| async { Err::<(), _>("fail".into()) })
                .await
                .is_err()
        );

        // Second failure - should open
        assert!(
            breaker
                .execute(|| async { Err::<(), _>("fail".into()) })
                .await
                .is_err()
        );

        // Should be open now
        assert!(matches!(
            breaker.execute(|| async { Ok::<(), _>(()) }).await,
            Err(CircuitBreakerError::Open)
        ));
    }

    #[tokio::test]
    async fn test_backpressure() {
        let controller = BackpressureController::new(2);

        assert!(controller.record_enqueue().is_ok());
        assert!(controller.record_enqueue().is_ok());

        // Should fail on third
        assert!(matches!(
            controller.record_enqueue(),
            Err(BackpressureError::QueueFull)
        ));
    }
}

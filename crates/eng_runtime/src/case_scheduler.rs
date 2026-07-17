use std::thread;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScheduledRow<T> {
    pub row_index: usize,
    pub worker_slot: usize,
    pub value: T,
}

pub(crate) fn effective_worker_count(
    scheduler: &str,
    configured_workers: usize,
    row_count: usize,
) -> usize {
    if row_count == 0 {
        0
    } else if scheduler == "parallel" {
        configured_workers.max(1).min(row_count)
    } else {
        1
    }
}

pub(crate) fn schedule_rows<T, F>(
    scheduler: &str,
    configured_workers: usize,
    row_count: usize,
    evaluate: F,
) -> Vec<ScheduledRow<T>>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    let worker_count = effective_worker_count(scheduler, configured_workers, row_count);
    if worker_count <= 1 {
        return (0..row_count)
            .map(|row_index| ScheduledRow {
                row_index,
                worker_slot: 1,
                value: evaluate(row_index),
            })
            .collect();
    }

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let rows_per_worker = row_count / worker_count;
            let extra_rows = row_count % worker_count;
            let start = worker_index * rows_per_worker + worker_index.min(extra_rows);
            let end = start + rows_per_worker + usize::from(worker_index < extra_rows);
            let evaluate = &evaluate;
            handles.push(scope.spawn(move || {
                (start..end)
                    .map(|row_index| ScheduledRow {
                        row_index,
                        worker_slot: worker_index + 1,
                        value: evaluate(row_index),
                    })
                    .collect::<Vec<_>>()
            }));
        }

        let mut rows = Vec::with_capacity(row_count);
        for handle in handles {
            rows.extend(
                handle
                    .join()
                    .expect("native case scheduler worker must not panic"),
            );
        }
        rows
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    use super::*;

    #[test]
    fn parallel_scheduler_preserves_row_order_and_static_worker_slots() {
        let rows = schedule_rows("parallel", 3, 8, |row| row * 10);
        assert_eq!(
            rows.iter()
                .map(|row| (row.row_index, row.worker_slot, row.value))
                .collect::<Vec<_>>(),
            vec![
                (0, 1, 0),
                (1, 1, 10),
                (2, 1, 20),
                (3, 2, 30),
                (4, 2, 40),
                (5, 2, 50),
                (6, 3, 60),
                (7, 3, 70),
            ]
        );
    }

    #[test]
    fn parallel_scheduler_executes_rows_concurrently() {
        let barrier = Arc::new(Barrier::new(4));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum_active = Arc::new(AtomicUsize::new(0));

        let rows = schedule_rows("parallel", 4, 4, {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let maximum_active = Arc::clone(&maximum_active);
            move |row| {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum_active.fetch_max(current, Ordering::SeqCst);
                barrier.wait();
                active.fetch_sub(1, Ordering::SeqCst);
                row
            }
        });

        assert_eq!(rows.len(), 4);
        assert_eq!(maximum_active.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn effective_workers_all_receive_rows_for_uneven_partitions() {
        let rows = schedule_rows("parallel", 4, 5, |row| row);
        assert_eq!(effective_worker_count("parallel", 4, 5), 4);
        assert_eq!(
            rows.iter().map(|row| row.worker_slot).collect::<Vec<_>>(),
            vec![1, 1, 2, 3, 4]
        );
    }

    #[test]
    fn sequential_scheduler_does_not_honor_extra_workers() {
        let rows = schedule_rows("sequential", 8, 3, |row| row);
        assert!(rows.iter().all(|row| row.worker_slot == 1));
        assert_eq!(effective_worker_count("sequential", 8, 3), 1);
    }
}

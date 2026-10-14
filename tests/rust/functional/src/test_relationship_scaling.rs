//! The cost of validating a relationship write must follow the write, not the store.
//!
//! 2.0.0 checked every write to an exclusive relationship (one-to-one,
//! one-to-many, ordered one-to-many) by copying the targets of every *other*
//! row of that junction into a fresh set, even when the write carried no
//! target at all. Creating N tasks that each own one comment therefore cost
//! O(N^2): each create and each set walked all the rows written before it.
//!
//! These tests pin the fix without trusting wall-clock time. They count the
//! bytes one write allocates, a deterministic measure of work that the old
//! check made proportional to the whole junction, and compare a small store
//! with one 64 times larger. The benchmark at the bottom does use time, so it
//! is `#[ignore]`d and only reports.

use crate::alloc_counter::bytes_allocated_by;
use common::database::db_context::DbContext;
use common::database::transactions::Transaction;
use common::direct_access::repository_factory;
use common::direct_access::task::TaskRelationshipField;
use common::entities::{Comment, Task};
use common::event::EventBuffer;
use common::types::EntityId;

/// Fill a store with `n` tasks that each own one comment, in batches so that
/// the setup itself stays linear whatever the validation costs.
fn populate(db: &DbContext, n: usize) {
    let mut tx = Transaction::begin_write_transaction(db).unwrap();
    let mut events = EventBuffer::new();
    let mut tasks = repository_factory::write::create_task_repository(&tx).unwrap();
    let mut comments = repository_factory::write::create_comment_repository(&tx).unwrap();
    let created_tasks = tasks
        .create_orphan_multi(&mut events, &vec![Task::default(); n])
        .unwrap();
    let created_comments = comments
        .create_orphan_multi(&mut events, &vec![Comment::default(); n])
        .unwrap();
    tasks
        .set_relationship_multi(
            &mut events,
            &TaskRelationshipField::Comments,
            created_tasks
                .iter()
                .zip(&created_comments)
                .map(|(task, comment)| (task.id, vec![comment.id]))
                .collect(),
        )
        .unwrap();
    drop(tasks);
    drop(comments);
    tx.commit().unwrap();
}

/// Bytes allocated by (a) creating a task, whose empty `comments` list is
/// still validated, and (b) giving a fresh task a fresh comment, in a store
/// already holding `n` tasks with one comment each. Each is the least of a few
/// tries: the maps are persistent tries shared with the transaction's
/// savepoint, so how much of a path a write copies varies with the hashes.
fn bytes_for_one_write(n: usize) -> (usize, usize) {
    const TRIES: usize = 5;
    let db = DbContext::new().unwrap();
    populate(&db, n);
    let mut tx = Transaction::begin_write_transaction(&db).unwrap();
    let mut events = EventBuffer::new();
    let mut tasks = repository_factory::write::create_task_repository(&tx).unwrap();
    let mut comments = repository_factory::write::create_comment_repository(&tx).unwrap();
    let mut create_bytes = usize::MAX;
    let mut set_bytes = usize::MAX;
    for _ in 0..TRIES {
        let comment = comments
            .create_orphan(&mut events, &Comment::default())
            .unwrap();
        let (task, bytes) =
            bytes_allocated_by(|| tasks.create_orphan(&mut events, &Task::default()).unwrap());
        create_bytes = create_bytes.min(bytes);
        let ((), bytes) = bytes_allocated_by(|| {
            tasks
                .set_relationship(
                    &mut events,
                    &task.id,
                    &TaskRelationshipField::Comments,
                    &[comment.id],
                )
                .unwrap()
        });
        set_bytes = set_bytes.min(bytes);
    }
    drop(tasks);
    drop(comments);
    tx.commit().unwrap();
    db.get_store().check_junction_indexes().unwrap();
    (create_bytes, set_bytes)
}

#[test]
fn an_exclusive_write_allocates_independently_of_the_junction_size() {
    const SMALL: usize = 256;
    const LARGE: usize = SMALL * 64;
    let (small_create, small_set) = bytes_for_one_write(SMALL);
    let (large_create, large_set) = bytes_for_one_write(LARGE);
    println!(
        "bytes allocated by one create / set_relationship: {small_create} / {small_set} beside \
         {SMALL} tasks, {large_create} / {large_set} beside {LARGE}"
    );
    // Any check that gathers the junction's other targets allocates at least
    // one id per row it walks. Between these two stores that is more than
    // `(LARGE - SMALL) * 8` bytes; the 2.0.0 check grew by about 600 KB. What
    // may grow is a write's copy of one path through the persistent maps,
    // which is a few KB deeper in the larger store.
    let one_id_per_added_row = (LARGE - SMALL) * std::mem::size_of::<EntityId>();
    for (what, small, large) in [
        ("create", small_create, large_create),
        ("set_relationship", small_set, large_set),
    ] {
        assert!(
            large.saturating_sub(small) < one_id_per_added_row,
            "{what} allocated {large} bytes beside {LARGE} tasks but {small} beside {SMALL}: \
             validating one write is walking the whole junction again"
        );
    }
}

/// Item-by-item load: create a task, create its comment, link them. This is
/// the shape of a project load, where creates and sets interleave, so nothing
/// can be batched or computed once.
fn interleaved_load(n: usize) -> std::time::Duration {
    let db = DbContext::new().unwrap();
    let mut tx = Transaction::begin_write_transaction(&db).unwrap();
    let mut events = EventBuffer::new();
    let mut tasks = repository_factory::write::create_task_repository(&tx).unwrap();
    let mut comments = repository_factory::write::create_comment_repository(&tx).unwrap();
    let started = std::time::Instant::now();
    let mut last: EntityId = 0;
    for _ in 0..n {
        let task = tasks.create_orphan(&mut events, &Task::default()).unwrap();
        let comment = comments
            .create_orphan(&mut events, &Comment::default())
            .unwrap();
        tasks
            .set_relationship(
                &mut events,
                &task.id,
                &TaskRelationshipField::Comments,
                &[comment.id],
            )
            .unwrap();
        last = task.id;
    }
    let elapsed = started.elapsed();
    drop(tasks);
    drop(comments);
    tx.commit().unwrap();
    assert_eq!(last as usize, n);
    db.get_store().check_junction_indexes().unwrap();
    elapsed
}

/// `cargo test -p functional-tests bench_interleaved_load -- --ignored --nocapture`
/// Sizes come from `QLEANY_BENCH_SIZES` (comma-separated), default 1000,4000,16000.
#[test]
#[ignore = "benchmark: reports timings, asserts nothing about them"]
fn bench_interleaved_load() {
    let sizes: Vec<usize> = std::env::var("QLEANY_BENCH_SIZES")
        .unwrap_or_else(|_| "1000,4000,16000".into())
        .split(',')
        .map(|s| {
            s.trim()
                .parse()
                .expect("QLEANY_BENCH_SIZES: comma-separated sizes")
        })
        .collect();
    for n in sizes {
        let elapsed = interleaved_load(n);
        println!(
            "interleaved load of {n:>6} tasks with one comment each: {:>9.3} s ({:.2} us per task)",
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() * 1e6 / n as f64
        );
    }
}

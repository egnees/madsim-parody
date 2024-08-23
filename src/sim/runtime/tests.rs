use super::Runtime;

#[test]
fn basic() {
    let runtime = Runtime::new();
    let handle = runtime.spawn(async { 5 });
    runtime.spawn(async {
        let x = handle.await.unwrap();
        assert_eq!(x, 5);
    });
    assert!(runtime.next_step());
    assert!(runtime.next_step());
    assert!(!runtime.next_step());
}

#[test]
fn steps_correct() {
    let runtime = Runtime::new();
    let task = runtime.spawn(async { 5 });
    runtime.spawn(async {
        let x = task.await.unwrap();
        assert_eq!(x, 5);
    });
    let steps = runtime.make_steps(None);
    assert_eq!(steps, 2);
    runtime.spawn(async { 10 });
    runtime.spawn(async { 20 });
    runtime.spawn(async { 30 });
    let steps = runtime.make_steps(Some(2));
    assert_eq!(steps, 2);
    assert!(runtime.next_step());
    assert!(!runtime.next_step());
}

#[test]
fn tasks_reordering() {
    let runtime = Runtime::new();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = runtime.spawn(async move {
        let result = receiver.await.unwrap();
        result
    });
    runtime.spawn(async move {
        sender.send(1).unwrap();
        let result = handle.await.unwrap();
        assert_eq!(result, 1);
    });
    let steps = runtime.make_steps(None);
    assert_eq!(steps, 4);
}

#[test]
fn handle_lifetime() {
    let runtime = Runtime::new();
    let handle = runtime.spawn(async { 5 });
    runtime.make_steps(None);
    runtime.spawn(async {
        let result = handle.await;
        assert_eq!(result, Ok(5));
    });
    runtime.make_steps(None);
}

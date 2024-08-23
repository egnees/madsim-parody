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
}
